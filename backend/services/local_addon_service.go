package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/util"
	"os/exec"
	"path/filepath"
)

type LocalAddonService struct{}

func (s *LocalAddonService) OpenDirectory(name string) error {
	return exec.Command("explorer", filepath.Join(config.GetAddonDir(), name)).Start()
}

func (s *LocalAddonService) GetAddOns() []addon.Addon {
	return addon.GetAddons()
}

func (s *LocalAddonService) IsInstalled(name string) bool {
	return addon.IsInstalled(name)
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

	return addon.RemoveManagedAddon(name) && addon.RemoveFromAddonsTxt(name)
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
