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

func ReadAddonsTxt() []string {
	lines, err := file.ReadLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
	if err != nil {
		logger.Error("Error reading addons.txt:", err)
		return nil
	}

	// Remove "AddonUpdateNotification" from lines if it exists
	if idx := slices.Index(lines, "AddonUpdateNotification"); idx >= 0 {
		lines = slices.Delete(lines, idx, idx+1)
	}

	installedAddonNamesMu.Lock()
	installedAddonNames = lines
	installedAddonNamesMu.Unlock()
	return lines
}

func AddToAddonsTxt(addonName string) bool {
	lines := ReadAddonsTxt()
	// Check if addon is already in addons.txt
	if slices.Contains(lines, addonName) {
		return true
	}

	lines = append(lines, addonName)
	err := file.WriteLines(filepath.Join(config.GetString("general.aacPath"), "Addon", "addons.txt"), lines)
	if err != nil {
		logger.Error("Error adding addon to addons.txt:", err)
		return false
	}

	installedAddonNames = lines
	return true
}

func CreateAddonsTxt() {
	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), []string{})
	if err != nil {
		dialog.Message("Error occurred while creating addons.txt: %s", err.Error()).Title("Classic Addon Manager Error").Error()
		logger.Fatal("Error creating addons.txt:", err)
		return
	}
	installedAddonNamesMu.Lock()
	installedAddonNames = []string{}
	installedAddonNamesMu.Unlock()
}

func RemoveFromAddonsTxt(addonName string) bool {
	lines := ReadAddonsTxt()
	for i, line := range lines {
		if line == addonName {
			lines = slices.Delete(lines, i, i+1)
			break
		}
	}

	err := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), lines)
	if err != nil {
		logger.Error("Error removing addon from addons.txt:", err)
		return false
	}

	installedAddonNames = lines
	return true
}
