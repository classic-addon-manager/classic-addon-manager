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
	writeAddonsTxtLines   = file.WriteLines
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

func addonsTxtPath() (string, error) {
	addonDir, err := config.GetAddonDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(addonDir, "addons.txt"), nil
}

func ReadAddonsTxt() ([]string, error) {
	path, err := addonsTxtPath()
	if err != nil {
		logger.Error("Error resolving addons.txt path:", err)
		return nil, err
	}

	lines, err := readAddonsTxtLines(path)
	if err != nil {
		logger.Error("Error reading addons.txt:", err)
		return nil, err
	}

	setInstalledAddonNames(lines)

	// Remove "AddonUpdateNotification" from lines if it exists
	return withoutUpdateNotification(lines), nil
}

// writeAddonsTxtLocked writes the full list, including any update notification.
// The caller must hold installedAddonNamesMu.
func writeAddonsTxtLocked(names []string) error {
	path, err := addonsTxtPath()
	if err != nil {
		return err
	}

	installedAddonNames = names
	if err := writeAddonsTxtLines(path, names); err != nil {
		lines, readErr := readAddonsTxtLines(path)
		if readErr != nil {
			logger.Error("Error re-reading addons.txt after failed write:", readErr)
		} else {
			installedAddonNames = lines
		}
		return err
	}
	return nil
}

func AddToAddonsTxt(addonName string) error {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()

	_, err := addonsTxtPath()
	if err != nil {
		logger.Error("Error resolving addons.txt path:", err)
		return err
	}

	// Check if addon is already in addons.txt
	if slices.Contains(installedAddonNames, addonName) {
		return nil // Already exists, nothing to do
	}

	err = writeAddonsTxtLocked(append(installedAddonNames, addonName))
	if err != nil {
		logger.Error("Error adding addon to addons.txt:", err)
	}
	return err
}

func CreateAddonsTxt() error {
	path, err := addonsTxtPath()
	if err != nil {
		logger.Error("Error resolving addons.txt path:", err)
		return err
	}

	err = file.WriteLines(path, []string{})
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

	_, err := addonsTxtPath()
	if err != nil {
		logger.Error("Error resolving addons.txt path:", err)
		return err
	}

	// Find and remove addon
	idx := slices.Index(installedAddonNames, addonName)
	if idx < 0 {
		return nil
	}
	names := slices.Delete(slices.Clone(installedAddonNames), idx, idx+1)
	err = writeAddonsTxtLocked(names)
	if err != nil {
		logger.Error("Error removing addon from addons.txt:", err)
	}
	return err
}
