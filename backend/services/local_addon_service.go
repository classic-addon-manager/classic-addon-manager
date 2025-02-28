package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"path/filepath"
)

type LocalAddonService struct{}

func (s *LocalAddonService) GetAddOns() []addon.Addon {
	return addon.GetAddons()
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
