package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"

	"path/filepath"
	"slices"
	"sync"

	"github.com/sqweek/dialog"
)

var (
	installedAddonNames   []string
	installedAddonNamesMu sync.RWMutex
)

func GetInstalledAddonNames() []string {
	installedAddonNamesMu.RLock()
	defer installedAddonNamesMu.RUnlock()
	return installedAddonNames
}

func setInstalledAddonNames(names []string) {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()
	installedAddonNames = names
}

func ReadAddonsTxt() ([]string, error) {
	lines, err := file.ReadLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
	if err != nil {
		logger.Error("Error reading addons.txt:", err)
		return nil, err
	}

	// Remove "AddonUpdateNotification" from lines if it exists
	if idx := slices.Index(lines, "AddonUpdateNotification"); idx >= 0 {
		lines = slices.Delete(lines, idx, idx+1)
	}

	setInstalledAddonNames(lines)
	return lines, nil
}

func AddToAddonsTxt(addonName string) error {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()

	// Check if addon is already in addons.txt
	if slices.Contains(installedAddonNames, addonName) {
		return nil // Already exists, nothing to do
	}

	// Add to slice
	installedAddonNames = append(installedAddonNames, addonName)

	// Write to file
	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), installedAddonNames)
	if err != nil {
		logger.Error("Error adding addon to addons.txt:", err)
		// Rollback the in-memory change if file write fails
		lines, readErr := file.ReadLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
		if readErr != nil {
			logger.Error("Error re-reading addons.txt after failed write:", readErr)
			return err
		}
		installedAddonNames = lines
		return err
	}

	return nil
}

func CreateAddonsTxt() {
	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), []string{})
	if err != nil {
		dialog.Message("Error occurred while creating addons.txt: %s", err.Error()).Title("Classic Addon Manager Error").Error()
		logger.Fatal("Error creating addons.txt:", err)
		return
	}
	setInstalledAddonNames([]string{})
}

func RemoveFromAddonsTxt(addonName string) error {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()

	// Find and remove addon
	found := false
	for i, name := range installedAddonNames {
		if name == addonName {
			installedAddonNames = slices.Delete(installedAddonNames, i, i+1)
			found = true
			break
		}
	}

	// If not found, nothing to do
	if !found {
		return nil
	}

	// Write to file
	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), installedAddonNames)
	if err != nil {
		logger.Error("Error removing addon from addons.txt:", err)
		// Rollback the in-memory change if file write fails
		lines, readErr := file.ReadLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
		if readErr != nil {
			logger.Error("Error re-reading addons.txt after failed write:", readErr)
			return err
		}
		installedAddonNames = lines
		return err
	}

	return nil
}
