package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"fmt"
	"sort"
)

type RemoteAddonService struct{}

const maxDependencyDepth = 10

// installAddon is the inner installer used by InstallAddon. Tests replace it
// so (false, nil) can be stubbed without HTTP or the user addon dir.
var installAddon = addon.InstallAddon

func (s *RemoteAddonService) GetAddonManifest() []shared.AddonManifest {
	return addon.GetAddonManifest()
}

func (s *RemoteAddonService) InstallAddon(ad shared.AddonManifest, version string) (bool, error) {
	ok, err := installAddon(ad, version)
	if err != nil {
		logger.Error("Error installing addon:", err)
		return false, err
	}

	return ok, nil
}

func (s *RemoteAddonService) UpdateAddon(ad shared.AddonManifest, version string) (shared.InstallWithDependenciesResult, error) {
	resolution, err := s.ResolveDependencies(ad)
	if err != nil {
		return shared.InstallWithDependenciesResult{}, err
	}

	return applyDependenciesThenParent(
		ad,
		resolution,
		s.InstallAddon,
		func() (bool, error) { return addon.UpdateAddon(ad, version) },
		addon.SortAddonsTxt,
	), nil
}

func (s *RemoteAddonService) GetLatestRelease(name string) (api.Release, error) {
	release, err := api.GetAddonRelease(name, "latest")
	if err != nil {
		logger.Error("Error getting latest release:", err)
		return api.Release{}, err
	}
	return release, nil
}

func (s *RemoteAddonService) CheckAddonUpdatesBulk(names []string) (map[string]api.Release, error) {
	releases, err := api.GetLatestReleasesBulk(names)
	if err != nil {
		logger.Error("Service: Error from GetLatestReleasesBulk:", err)
		return nil, err
	}
	return releases, nil
}

func (s *RemoteAddonService) GetSubscribedAddons() ([]shared.AddonManifest, error) {
	return api.GetSubscribedAddons()
}

func (s *RemoteAddonService) ResolveDependencies(ad shared.AddonManifest) (shared.DependencyResolutionResult, error) {
	result := shared.DependencyResolutionResult{
		Dependencies: []shared.DependencyInfo{},
		Errors:       []string{},
	}

	manifests := addon.GetAddonManifest()
	if len(manifests) == 0 {
		return result, fmt.Errorf("failed to fetch addon manifests")
	}

	manifestByName := make(map[string]shared.AddonManifest, len(manifests))
	for _, manifest := range manifests {
		manifestByName[manifest.Name] = manifest
	}

	if _, err := addon.ReadAddonsTxt(); err != nil {
		logger.Error("ResolveDependencies: failed to read addons.txt:", err)
	}

	installedByName := make(map[string]struct{})
	for _, name := range addon.GetInstalledAddonNames() {
		installedByName[name] = struct{}{}
	}

	dependencyByName := make(map[string]shared.DependencyInfo)

	var walk func(name string, depth int, lineage map[string]struct{})
	walk = func(name string, depth int, lineage map[string]struct{}) {
		if depth > maxDependencyDepth {
			result.Errors = append(result.Errors, fmt.Sprintf("Maximum dependency depth exceeded for: %s", name))
			return
		}

		if _, circular := lineage[name]; circular {
			result.Errors = append(result.Errors, fmt.Sprintf("Circular dependency detected: %s", name))
			return
		}

		manifest, ok := manifestByName[name]
		if !ok {
			result.Errors = append(result.Errors, fmt.Sprintf("Addon %s not found in repository manifests", name))
			return
		}

		_, isInstalled := installedByName[name]
		existing, exists := dependencyByName[name]
		if !exists || depth > existing.Depth {
			dependencyByName[name] = shared.DependencyInfo{
				Manifest:    manifest,
				IsInstalled: isInstalled,
				Depth:       depth,
			}
		}

		nextLineage := make(map[string]struct{}, len(lineage)+1)
		for key := range lineage {
			nextLineage[key] = struct{}{}
		}
		nextLineage[name] = struct{}{}

		for _, depName := range manifest.Dependencies {
			walk(depName, depth+1, nextLineage)
		}
	}

	rootLineage := map[string]struct{}{
		ad.Name: {},
	}
	for _, depName := range ad.Dependencies {
		walk(depName, 0, rootLineage)
	}

	dependencies := make([]shared.DependencyInfo, 0, len(dependencyByName))
	for _, dependency := range dependencyByName {
		dependencies = append(dependencies, dependency)
	}

	sort.SliceStable(dependencies, func(i, j int) bool {
		if dependencies[i].Depth != dependencies[j].Depth {
			return dependencies[i].Depth > dependencies[j].Depth
		}
		return dependencies[i].Manifest.Name < dependencies[j].Manifest.Name
	})

	result.Dependencies = dependencies
	return result, nil
}

func (s *RemoteAddonService) InstallAddonWithDependencies(ad shared.AddonManifest, version string) (shared.InstallWithDependenciesResult, error) {
	resolution, err := s.ResolveDependencies(ad)
	if err != nil {
		return shared.InstallWithDependenciesResult{}, err
	}

	return applyDependenciesThenParent(
		ad,
		resolution,
		s.InstallAddon,
		func() (bool, error) { return s.InstallAddon(ad, version) },
		addon.SortAddonsTxt,
	), nil
}

// installDep, parent, and sort failures are recorded on the returned result.
// Only ResolveDependencies in the callers returns a Go error.
func applyDependenciesThenParent(
	ad shared.AddonManifest,
	resolution shared.DependencyResolutionResult,
	installDep func(shared.AddonManifest, string) (bool, error),
	parent func() (bool, error),
	sortAddons func() error,
) shared.InstallWithDependenciesResult {
	result := shared.InstallWithDependenciesResult{
		Success:            false,
		DependencyWarnings: resolution.Errors,
		Dependencies:       []shared.AddonInstallStatus{},
		MainAddon: shared.AddonInstallStatus{
			Name:  ad.Name,
			Alias: ad.Alias,
		},
	}

	for _, dep := range resolution.Dependencies {
		status := shared.AddonInstallStatus{
			Name:  dep.Manifest.Name,
			Alias: dep.Manifest.Alias,
		}

		if status.Alias == "" {
			status.Alias = dep.Manifest.Name
		}

		if dep.IsInstalled {
			status.Success = true
			status.Skipped = true
			result.Dependencies = append(result.Dependencies, status)
			continue
		}

		ok, installErr := installDep(dep.Manifest, "latest")
		if installErr != nil || !ok {
			status.Success = false
			if installErr != nil {
				status.Error = installErr.Error()
			} else {
				status.Error = "installation failed"
			}
			result.Dependencies = append(result.Dependencies, status)
			result.MainAddon.Error = "dependency installation failed"
			return result
		}

		status.Success = true
		result.Dependencies = append(result.Dependencies, status)
	}

	ok, parentErr := parent()
	if parentErr != nil || !ok {
		result.MainAddon.Success = false
		if parentErr != nil {
			result.MainAddon.Error = parentErr.Error()
		} else {
			result.MainAddon.Error = "installation failed"
		}
		return result
	}

	result.MainAddon.Success = true
	result.Success = true

	if sortErr := sortAddons(); sortErr != nil {
		result.DependencyWarnings = append(result.DependencyWarnings, sortErr.Error())
		result.MainAddon.Error = sortErr.Error()
	}

	return result
}
