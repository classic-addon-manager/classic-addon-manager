package addon

import (
	"ClassicAddonManager/config"
	"ClassicAddonManager/file"
	"ClassicAddonManager/logger"
	"github.com/sqweek/dialog"
	"path/filepath"
)

var installedAddonNames []string

func GetInstalledAddonNames() []string {
	return installedAddonNames
}

func ReadAddonsTxt() []string {
	lines, err := file.ReadLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
	if err != nil {
		logger.Error("Error reading addons.txt:", err)
		return nil
	}
	installedAddonNames = lines
	return lines
}

func AddToAddonsTxt(addonName string) bool {
	lines := ReadAddonsTxt()
	// Check if addon is already in addons.txt
	for _, line := range lines {
		if line == addonName {
			return true
		}
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
	addonDir := config.GetAddonDir()
	err := file.WriteLines(filepath.Join(addonDir, "addons.txt"), nil)
	if err != nil {
		dialog.Message("Error occurred while creating addons.txt: %s", err.Error()).Title("Classic Addon Manager Error").Error()
		logger.Fatal("Error creating addons.txt:", err)
		return
	}
	installedAddonNames = []string{}
}

func RemoveFromAddonsTxt(addonName string) bool {
	lines := ReadAddonsTxt()
	for i, line := range lines {
		if line == addonName {
			lines = append(lines[:i], lines[i+1:]...)
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
