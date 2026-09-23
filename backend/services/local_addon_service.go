package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/util"
	"fmt"
	"path/filepath"
	"strings"
)

type LocalAddonService struct{}

var (
	isAddonInstalled     = addon.IsInstalled
	findLocalAddonByName = addon.FindLocalAddonByName
	removeFromAddonsTxt  = addon.RemoveFromAddonsTxt
	removeAddonDirectory = file.RemoveDir
	removeManagedAddon   = addon.RemoveManagedAddon
	sortAddonsTxt        = addon.SortAddonsTxt
	unsubscribeFromAddon = api.UnsubscribeFromAddon
	openAddonDirectory   = file.OpenDirectory
)

func (s *LocalAddonService) OpenDirectory(name string) error {
	if name == "." || !filepath.IsLocal(name) || strings.ContainsAny(name, `/\`) {
		return fmt.Errorf("invalid addon directory: %q", name)
	}

	root, err := resolveAddonDirectory(config.GetAddonDir())
	if err != nil {
		return err
	}
	target, err := resolveAddonDirectory(filepath.Join(root, name))
	if err != nil {
		return err
	}
	relative, err := filepath.Rel(root, target)
	if err != nil {
		return err
	}
	if relative == "." || !filepath.IsLocal(relative) {
		return fmt.Errorf("addon directory escapes addon root: %q", name)
	}
	return openAddonDirectory(target)
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
	if !isAddonInstalled(name) {
		return false
	}

	wasManaged := findLocalAddonByName(name) != nil

	if err := removeFromAddonsTxt(name); err != nil {
		logger.Error("Error removing addon from addons.txt:", err)
		return false
	}

	ok, err := removeAddonDirectory(filepath.Join(config.GetAddonDir(), name))
	if err != nil {
		logger.Error("Error removing addon directory:", err)
		return false
	}
	if !ok {
		return false
	}

	if wasManaged {
		removeManagedAddon(name)
	}

	// Reorder remaining addons so dependencies still load before dependents. A
	// cycle error is logged inside SortAddonsTxt; the removal itself succeeded.
	_ = sortAddonsTxt()

	if wasManaged {
		unsubscribeFromAddon(name)
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
