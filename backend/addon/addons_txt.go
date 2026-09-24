package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"

	"path/filepath"
	"slices"
	"strings"
	"sync"
)

var (
	installedAddonNames   []string
	installedAddonNamesMu sync.RWMutex
)

func GetInstalledAddonNames() []string {
	installedAddonNamesMu.RLock()
	defer installedAddonNamesMu.RUnlock()
	return withoutUpdateNotification(installedAddonNames)
}

func withoutUpdateNotification(names []string) []string {
	filtered := make([]string, 0, len(names))
	for _, name := range names {
		if name != "AddonUpdateNotification" {
			filtered = append(filtered, name)
		}
	}
	return filtered
}

func setInstalledAddonNames(names []string) {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()
	installedAddonNames = names
}

func readAddonsTxtLines(path string) ([]string, error) {
	lines, err := file.ReadLines(path)
	if err != nil {
		return nil, err
	}

	names := make([]string, 0, len(lines))
	for _, line := range lines {
		if strings.TrimSpace(line) == "" {
			continue
		}
		names = append(names, line)
	}
	return names, nil
}

func ReadAddonsTxt() ([]string, error) {
	lines, err := readAddonsTxtLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
	if err != nil {
		logger.Error("Error reading addons.txt:", err)
		return nil, err
	}

	setInstalledAddonNames(lines)

	// Remove "AddonUpdateNotification" from lines if it exists
	return withoutUpdateNotification(lines), nil
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
		lines, readErr := readAddonsTxtLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
		if readErr != nil {
			logger.Error("Error re-reading addons.txt after failed write:", readErr)
			return err
		}
		installedAddonNames = lines
		return err
	}

	return nil
}

func CreateAddonsTxt() error {
	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), []string{})
	if err != nil {
		logger.Error("Error creating addons.txt:", err)
		return err
	}
	setInstalledAddonNames([]string{})
	return nil
}

func RemoveFromAddonsTxt(addonName string) error {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()

	// Find and remove addon
	idx := slices.Index(installedAddonNames, addonName)
	if idx < 0 {
		return nil
	}
	installedAddonNames = slices.Delete(installedAddonNames, idx, idx+1)

	// Write to file
	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), installedAddonNames)
	if err != nil {
		logger.Error("Error removing addon from addons.txt:", err)
		// Rollback the in-memory change if file write fails
		lines, readErr := readAddonsTxtLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
		if readErr != nil {
			logger.Error("Error re-reading addons.txt after failed write:", readErr)
			return err
		}
		installedAddonNames = lines
		return err
	}

	return nil
}
