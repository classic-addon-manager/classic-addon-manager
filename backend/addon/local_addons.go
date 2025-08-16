package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/util"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"
)

type Addon struct {
	Name        string    `json:"name"`
	Alias       string    `json:"alias"`
	Description string    `json:"description"`
	Version     string    `json:"version"`
	Commit      string    `json:"commit"`
	Author      string    `json:"author"`
	Repo        string    `json:"repo"`
	IsManaged   bool      `json:"isManaged"`
	UpdatedAt   time.Time `json:"updatedAt"`
	Branch      string    `json:"branch,omitempty"`
}

var LocalAddons map[string]Addon

func LoadManagedAddonsFile() error {
	LocalAddons = make(map[string]Addon)
	if !file.FileExists(filepath.Join(config.GetDataDir(), "managed_addons.json")) {
		return errors.New("managed_addons.json not found")
	}

	data, err := os.ReadFile(filepath.Join(config.GetDataDir(), "managed_addons.json"))
	if err != nil {
		return err
	}

	var addons []Addon
	err = json.Unmarshal(data, &addons)
	if err != nil {
		return err
	}

	for _, addon := range addons {
		LocalAddons[addon.Name] = addon
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
		Name:        manifest.Name,
		Description: manifest.Description,
		Version:     release.TagName,
		Commit:      release.Tag.Sha,
		Author:      manifest.Author,
		Repo:        manifest.Repo,
		IsManaged:   true,
		UpdatedAt:   release.PublishedAt,
		Branch:      manifest.Branch,
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
		managedAddons = append(managedAddons, addon)
	}

	data, err := json.Marshal(managedAddons)
	if err != nil {
		logger.Error("Error marshalling managed addons:", err)
		return
	}

	fp := filepath.Join(config.GetDataDir(), "managed_addons.json")
	err = os.WriteFile(fp, data, 0644)
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

	AddToAddonsTxt(addonName)

	return addonName, nil
}
