package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/util"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"
)

const ManagedAddonsFileVersion = 1

type Addon struct {
	Name         string    `json:"name"`
	Alias        string    `json:"alias"`
	Description  string    `json:"description"`
	Version      string    `json:"version"`
	Commit       string    `json:"commit"`
	Author       string    `json:"author"`
	Repo         string    `json:"repo"`
	IsManaged    bool      `json:"isManaged"`
	UpdatedAt    time.Time `json:"updatedAt"`
	Branch       string    `json:"branch,omitempty"`
	Dependencies []string  `json:"dependencies"`
}

type ManagedAddonsFile struct {
	Version int     `json:"version"`
	Addons  []Addon `json:"addons"`
}

var LocalAddons map[string]Addon

func managedAddonsFilePath() string {
	return filepath.Join(config.GetDataDir(), "managed_addons.json")
}

func normalizeAddon(addon Addon) Addon {
	if addon.Dependencies == nil {
		addon.Dependencies = []string{}
	}
	return addon
}

func backfillDependencies(addons []Addon) []Addon {
	manifests := GetAddonManifest()
	if len(manifests) == 0 {
		logger.Warn("backfillDependencies: no addon manifests available, skipping dependency backfill")
		return addons
	}

	manifestByName := make(map[string]shared.AddonManifest, len(manifests))
	for _, manifest := range manifests {
		manifestByName[manifest.Name] = manifest
	}

	result := make([]Addon, len(addons))
	for i, addon := range addons {
		result[i] = normalizeAddon(addon)
		manifest, ok := manifestByName[addon.Name]
		if !ok {
			logger.Warn(fmt.Sprintf("backfillDependencies: manifest not found for addon: %s", addon.Name))
			continue
		}
		result[i].Dependencies = append([]string(nil), manifest.Dependencies...)
	}
	return result
}

func LoadManagedAddonsFile() error {
	LocalAddons = make(map[string]Addon)
	fp := managedAddonsFilePath()
	if !file.FileExists(fp) {
		return errors.New("managed_addons.json not found")
	}

	data, err := os.ReadFile(fp)
	if err != nil {
		return err
	}

	trimmed := bytes.TrimSpace(data)
	if len(trimmed) == 0 {
		return errors.New("managed_addons.json is empty")
	}

	if trimmed[0] == '[' {
		var addons []Addon
		if err := json.Unmarshal(data, &addons); err != nil {
			return err
		}

		addons = backfillDependencies(addons)
		for _, addon := range addons {
			LocalAddons[addon.Name] = normalizeAddon(addon)
		}

		backupPath := filepath.Join(config.GetDataDir(), "managed_addons.json.bak")
		if err := os.WriteFile(backupPath, data, 0644); err != nil {
			logger.Error("Error writing managed_addons.json backup:", err)
			return err
		}

		logger.Info("Migrated managed_addons.json from legacy format to version 1")
		SaveManagedAddonsToDisk()
		return nil
	}

	var managedFile ManagedAddonsFile
	if err := json.Unmarshal(data, &managedFile); err != nil {
		return err
	}

	if managedFile.Version != ManagedAddonsFileVersion {
		logger.Warn(fmt.Sprintf("managed_addons.json has unsupported version: %d", managedFile.Version))
	}

	for _, addon := range managedFile.Addons {
		LocalAddons[addon.Name] = normalizeAddon(addon)
	}

	return nil
}

func FindLocalAddonByName(name string) *Addon {
	if addon, exists := LocalAddons[name]; exists {
		return &addon
	}
	return nil
}

func IsInstalled(name string) bool {
	return slices.Contains(GetInstalledAddonNames(), name)
}

func AddManagedAddon(manifest shared.AddonManifest, release api.Release) {
	addon := Addon{
		Name:         manifest.Name,
		Description:  manifest.Description,
		Version:      release.TagName,
		Commit:       release.Tag.Sha,
		Author:       manifest.Author,
		Repo:         manifest.Repo,
		IsManaged:    true,
		UpdatedAt:    release.PublishedAt,
		Branch:       manifest.Branch,
		Dependencies: append([]string(nil), manifest.Dependencies...),
	}

	if manifest.Alias == "" {
		addon.Alias = strings.ReplaceAll(manifest.Name, "_", " ")
	} else {
		addon.Alias = manifest.Alias
	}

	LocalAddons[manifest.Name] = addon
	SaveManagedAddonsToDisk()
}

func RemoveManagedAddon(name string) bool {
	delete(LocalAddons, name)
	SaveManagedAddonsToDisk()
	// Check if the addon is still in LocalAddons
	if _, exists := LocalAddons[name]; exists {
		return false
	}
	return true
}

func SaveManagedAddonsToDisk() {
	managedAddons := make([]Addon, 0, len(LocalAddons))
	for _, addon := range LocalAddons {
		managedAddons = append(managedAddons, normalizeAddon(addon))
	}

	fileData := ManagedAddonsFile{
		Version: ManagedAddonsFileVersion,
		Addons:  managedAddons,
	}

	data, err := json.Marshal(fileData)
	if err != nil {
		logger.Error("Error marshalling managed addons:", err)
		return
	}

	err = os.WriteFile(managedAddonsFilePath(), data, 0644)
	if err != nil {
		logger.Error("Error writing managed addons to disk:", err)
		return
	}

	logger.Info("Managed addons saved to disk")
}

func InstallZip(zipPath string) (string, error) {
	fileName := filepath.Base(zipPath)
	addonName := strings.TrimSuffix(fileName, filepath.Ext(fileName))

	// Validate the zip file contains a main.lua file
	err := file.ValidateAddonZip(zipPath)
	if err != nil {
		return "", err
	}

	// Copy the zip file to the cache directory
	cachePath := filepath.Join(config.GetCacheDir(), addonName+".zip")
	err = file.MoveFile(zipPath, cachePath)
	if err != nil {
		logger.Error("failed to copy zip file to cache directory", err)
		return "", err
	}

	// Extract the zip file to the cache directory
	err = util.ExtractAddonRelease(addonName+".zip", addonName)
	if err != nil {
		return "", err
	}

	// Move the extracted addon from cache to the addon directory
	if !util.MoveAddonRelease(addonName) {
		return "", errors.New("failed to move addon release")
	}

	if err := AddToAddonsTxt(addonName); err != nil {
		logger.Error("Error adding addon to addons.txt:", err)
		return "", err
	}

	// Reorder addons.txt so dependencies load before dependents. A cycle error
	// is logged inside SortAddonsTxt; the import itself still succeeded.
	_ = SortAddonsTxt()

	return addonName, nil
}
