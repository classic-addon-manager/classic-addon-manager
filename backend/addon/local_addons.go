package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
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
