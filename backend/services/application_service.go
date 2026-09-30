package services

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/appstate"
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/updater"
	"fmt"
	"path/filepath"
	"runtime"

	"github.com/wailsapp/wails/v3/pkg/application"
)

type ApplicationService struct {
	App *application.App
}

func (s *ApplicationService) GetVersion() string {
	return shared.Version
}

func (s *ApplicationService) GetOS() string {
	return runtime.GOOS
}

func (s *ApplicationService) GetLatestRelease() (api.ApplicationRelease, error) {
	release, err := api.GetLatestApplicationRelease()
	if err != nil {
		logger.Error("Error getting latest application release:", err)
		return api.ApplicationRelease{}, err
	}
	return release, nil
}

func (s *ApplicationService) GetAuthSession() shared.AuthSessionState {
	return shared.AuthSessionState{Token: auth.GetToken()}
}

func (s *ApplicationService) SaveAuthToken(token string) error {
	return auth.SaveToDisk(token)
}

func (s *ApplicationService) ClearAuthToken() error {
	return auth.DeleteFromDisk()
}

// SelfUpdateSupported tells the frontend whether this installation can update itself.
func (s *ApplicationService) SelfUpdateSupported() bool {
	return updater.SelfUpdateSupported()
}

// ConsumeUpdateFailure returns and removes the report left behind by a failed update.
func (s *ApplicationService) ConsumeUpdateFailure() (string, error) {
	return updater.ConsumeUpdateFailure()
}

func (s *ApplicationService) SelfUpdate(updateURL string, checksum string) error {
	return updater.SelfUpdate(updateURL, checksum)
}

func (s *ApplicationService) SelectAndValidateDocsPath(title string) (string, error) {
	dialog := s.App.Dialog.OpenFileWithOptions(&application.OpenFileDialogOptions{
		Title:                title,
		CanChooseDirectories: true,
		CanChooseFiles:       false,
	})

	path, err := dialog.PromptForSingleSelection()
	if err != nil {
		logger.Error("Prompt for single selection failed", err)
		return "", err
	}

	// Ensure that there is an Addon folder in the selected directory
	if !file.FileExists(filepath.Join(path, "Addon")) {
		return "", fmt.Errorf("invalid AAC documents path, try a different path")
	}

	if err := config.SetString("general.aacpath", path); err != nil {
		return "", err
	}

	return path, nil
}

func (s *ApplicationService) SettingsSetAutoDetectPath(enabled bool) error {
	return config.SetBool("general.autodetectpath", enabled)
}

func (s *ApplicationService) GetUIPreferences() shared.UIPreferences {
	return shared.UIPreferences{
		AccentColor:   config.GetString(config.KeyUIAccentColor, ""),
		AddonViewMode: config.GetString(config.KeyUIAddonViewMode, ""),
	}
}

func (s *ApplicationService) SetAccentColor(id string) error {
	return config.SetString(config.KeyUIAccentColor, id)
}

func (s *ApplicationService) SetAddonViewMode(mode string) error {
	return config.SetString(config.KeyUIAddonViewMode, mode)
}

func (s *ApplicationService) GetConfig() map[string]any {
	return config.GetAll()
}

func (s *ApplicationService) OpenCacheDir() error {
	cacheDir, err := config.GetCacheDir()
	if err != nil {
		return err
	}
	return file.OpenDirectory(cacheDir)
}

func (s *ApplicationService) OpenDataDir() error {
	dataDir, err := config.GetDataDir()
	if err != nil {
		return err
	}
	return file.OpenDirectory(dataDir)
}

func (s *ApplicationService) ShouldShowKofiModal() bool {
	return appstate.ShouldShowKofiModal()
}

func (s *ApplicationService) RecordKofiModalShown() error {
	return appstate.RecordKofiModalShown()
}

func (s *ApplicationService) EnsureAppStateInitialized() error {
	return appstate.EnsureInitialized()
}
