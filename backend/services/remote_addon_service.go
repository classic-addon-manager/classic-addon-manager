package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
)

type RemoteAddonService struct{}

func (s *RemoteAddonService) GetAddonManifest() []shared.AddonManifest {
	return addon.GetAddonManifest()
}

func (s *RemoteAddonService) InstallAddon(ad shared.AddonManifest, version string) (bool, error) {
	_, err := addon.InstallAddon(ad, version)
	if err != nil {
		logger.Error("Error installing addon:", err)
		return false, err
	}

	err = addon.AddToAddonsTxt(ad.Name)
	if err != nil {
		logger.Error(ad.Name+" - Error adding addon to addons.txt: ", err)
		return false, err
	}

	return true, nil
}

func (s *RemoteAddonService) UpdateAddon(ad shared.AddonManifest, version string) (bool, error) {
	_, err := addon.UpdateAddon(ad, version)
	if err != nil {
		logger.Error("Error updating addon:", err)
		return false, err
	}

	return true, nil
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
