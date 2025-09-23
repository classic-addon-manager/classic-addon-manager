package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/util"
	"path/filepath"
)

type LocalAddonService struct{}

func (s *LocalAddonService) OpenDirectory(name string) error {
	return file.OpenDirectory(filepath.Join(config.GetAddonDir(), name))
}

func (s *LocalAddonService) GetAddOns() []addon.Addon {
	return addon.GetAddons()
}

func (s *LocalAddonService) IsInstalled(name string) bool {
	return addon.IsInstalled(name)
}

func (s *LocalAddonService) GetAllInstalledAddonNames() []string {
	return addon.GetInstalledAddonNames()
}

func (s *LocalAddonService) UninstallAddon(name string) bool {
	if !addon.IsInstalled(name) {
		return false
	}

	ok, err := file.RemoveDir(filepath.Join(config.GetAddonDir(), name))
	if err != nil {
		logger.Error("Error removing addon directory:", err)
		return false
	}
	if !ok {
		return false
	}

	// Remove from managed addons
	wasRemoved := addon.RemoveManagedAddon(name)

	// Remove from addons.txt
	if err := addon.RemoveFromAddonsTxt(name); err != nil {
		logger.Error("Error removing addon from addons.txt:", err)
		return false
	}

	if wasRemoved {
		api.UnsubscribeFromAddon(name)
	}

	return true
}

func (s *LocalAddonService) InstallZipAddon(zipPath string) (string, error) {
	return addon.InstallZip(zipPath)
}

func (s *LocalAddonService) UnmanageAddon(name string) bool {
	return addon.RemoveManagedAddon(name)
}

func (s *LocalAddonService) ResetSettings() error {
	err := addon.ResetAddonSettings()
	if err != nil {
		logger.Error("Error resetting addon settings:", err)
		return err
	}
	return nil
}

func (s *LocalAddonService) DiagnoseIssues() ([]util.LogParseResult, error) {
	return util.DiagnoseIssues()
}
