package services

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/util"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sync"

	"github.com/wailsapp/wails/v3/pkg/application"
)

var authMutex = &sync.Mutex{}

type ApplicationService struct {
	App *application.App
}

func (s *ApplicationService) GetVersion() string {
	return shared.Version
}

func (s *ApplicationService) GetLatestRelease() (api.ApplicationRelease, error) {
	release, err := api.GetLatestApplicationRelease()
	if err != nil {
		logger.Error("Error getting latest application release:", err)
		return api.ApplicationRelease{}, err
	}
	return release, nil
}

func (s *ApplicationService) SetAuthToken(token string) {
	authMutex.Lock()
	shared.AuthToken = token
	authMutex.Unlock()
}

func (s *ApplicationService) SelfUpdate(updateURL string) error {
	// Current exe path
	exePath, err := os.Executable()
	if err != nil {
		logger.Error("Error getting executable path:", err)
		return err
	}

	tmpDir := filepath.Join(os.TempDir(), "ClassicAddonManager")
	err = os.MkdirAll(tmpDir, 0755)
	if err != nil {
		return fmt.Errorf("error creating temporary directory: %s", err)
	}

	// Download the new version
	logger.Info(fmt.Sprintf("Downloading update from %s", updateURL))
	newExePath := filepath.Join(tmpDir, "ClassicAddonManager.new.exe")

	// Remove newExePath if it exists
	if _, err := os.Stat(newExePath); err == nil {
		err = os.Remove(newExePath)
		if err != nil {
			logger.Error("Error removing existing update file:", err)
			return err
		}
		logger.Info("Removed existing update file")
	}

	err = util.DownloadFile(updateURL, newExePath)
	if err != nil {
		logger.Error("Error downloading update:", err)
		return err
	}

	// Create update batch script
	scriptPath := filepath.Join(tmpDir, "update.bat")
	scriptContent := fmt.Sprintf(`@echo off
timeout /t 1 /nobreak > NUL
echo Updating Classic Addon Manager...
copy /Y "%s" "%s"
start "" "%s"
del "%s"
exit
`, newExePath, exePath, exePath, scriptPath)

	err = os.WriteFile(scriptPath, []byte(scriptContent), 0755)
	if err != nil {
		logger.Error("Error creating update script:", err)
		return err
	}

	// Run the update script and exit
	cmd := exec.Command("cmd", "/C", scriptPath)
	err = cmd.Start()
	if err != nil {
		logger.Error("Error starting update script:", err)
		return err
	}

	logger.Info("Update script started, exiting application.")
	os.Exit(0)

	return nil
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

	config.SetString("general.aacpath", path)

	return path, nil
}

func (s *ApplicationService) SettingsSetAutoDetectPath(enabled bool) {
	config.SetBool("general.autodetectpath", enabled)
}

func (s *ApplicationService) GetConfig() map[string]any {
	return config.GetAll()
}

func (s *ApplicationService) OpenCacheDir() error {
	return file.OpenDirectory(config.GetCacheDir())
}

func (s *ApplicationService) OpenDataDir() error {
	return file.OpenDirectory(config.GetDataDir())
}
