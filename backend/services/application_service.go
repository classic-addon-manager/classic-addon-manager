package services

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/appstate"
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/util"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"

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

// ErrSelfUpdateUnsupported is returned when the app cannot replace itself on the current platform.
var ErrSelfUpdateUnsupported = errors.New("automatic updates are only supported on Windows, please download the new version manually")

// Swapped out in tests so both the supported and unsupported paths can be exercised on any machine.
var selfUpdateOS = runtime.GOOS

const (
	updateReplaceAttempts = 30
	updateFailureFileName = "update-failed.txt"
)

func updateTempDir() string {
	return filepath.Join(os.TempDir(), "ClassicAddonManager")
}

// SelfUpdateSupported tells the frontend whether the Update Now action can work on this platform.
func (s *ApplicationService) SelfUpdateSupported() bool {
	return selfUpdateOS == "windows"
}

// ConsumeUpdateFailure returns the report left behind by a failed update, if any, and removes it
// so it is only shown once.
func (s *ApplicationService) ConsumeUpdateFailure() (string, error) {
	return consumeUpdateFailure(filepath.Join(updateTempDir(), updateFailureFileName))
}

func consumeUpdateFailure(path string) (string, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return "", nil
	}
	if err != nil {
		return "", err
	}
	if err := os.Remove(path); err != nil {
		logger.Error("Error removing update failure report:", err)
	}
	return strings.TrimSpace(string(data)), nil
}

// ErrUpdateVerificationFailed is returned when the downloaded update does not match the published checksum.
var ErrUpdateVerificationFailed = errors.New("the update download could not be verified, please try again later or download the new version manually")

// verifyUpdateChecksum compares the file's SHA-256 with the checksum published for the release.
// Releases published before checksums existed have none, so those are allowed through with a warning.
// A file that does not match is deleted so it can never be installed.
func verifyUpdateChecksum(path string, expected string) error {
	expected = strings.TrimSpace(expected)
	if expected == "" {
		logger.Warn("No checksum available for this update, skipping verification")
		return nil
	}

	actual, err := fileSHA256(path)
	if err != nil {
		logger.Error("Error hashing downloaded update:", err)
		removeUpdateFile(path)
		return ErrUpdateVerificationFailed
	}

	if !strings.EqualFold(actual, expected) {
		logger.Error("Update checksum mismatch:", fmt.Errorf("expected %s, got %s", strings.ToLower(expected), actual))
		removeUpdateFile(path)
		return ErrUpdateVerificationFailed
	}

	logger.Info("Update checksum verified: " + actual)
	return nil
}

func fileSHA256(path string) (string, error) {
	f, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()

	hash := sha256.New()
	if _, err := io.Copy(hash, f); err != nil {
		return "", err
	}
	return hex.EncodeToString(hash.Sum(nil)), nil
}

func removeUpdateFile(path string) {
	if err := os.Remove(path); err != nil && !errors.Is(err, os.ErrNotExist) {
		logger.Error("Error removing rejected update file:", err)
	}
}

func (s *ApplicationService) SelfUpdate(updateURL string, checksum string) error {
	if !s.SelfUpdateSupported() {
		return ErrSelfUpdateUnsupported
	}

	exePath, err := os.Executable()
	if err != nil {
		logger.Error("Error getting executable path:", err)
		return err
	}

	tmpDir := updateTempDir()
	err = os.MkdirAll(tmpDir, 0755)
	if err != nil {
		return fmt.Errorf("error creating temporary directory: %s", err)
	}

	failurePath := filepath.Join(tmpDir, updateFailureFileName)
	if err := os.Remove(failurePath); err != nil && !errors.Is(err, os.ErrNotExist) {
		logger.Error("Error removing old update failure report:", err)
	}

	logger.Info(fmt.Sprintf("Downloading update from %s", updateURL))
	newExePath := filepath.Join(tmpDir, "ClassicAddonManager.new.exe")

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

	if err := verifyUpdateChecksum(newExePath, checksum); err != nil {
		return err
	}

	scriptPath := filepath.Join(tmpDir, "update.bat")
	script := buildWindowsUpdateScript(windowsUpdateScript{
		NewExePath:  newExePath,
		TargetPath:  exePath,
		FailurePath: failurePath,
		Attempts:    updateReplaceAttempts,
		Relaunch:    true,
	})

	err = os.WriteFile(scriptPath, []byte(script), 0755)
	if err != nil {
		logger.Error("Error creating update script:", err)
		return err
	}

	cmd := exec.Command("cmd", "/C", scriptPath)
	err = cmd.Start()
	if err != nil {
		logger.Error("Error starting update script:", err)
		return err
	}

	logger.Info("Update script started, exiting application.")
	logger.Sync()
	os.Exit(0)

	return nil
}

type windowsUpdateScript struct {
	NewExePath  string
	TargetPath  string
	FailurePath string
	Attempts    int
	Relaunch    bool
}

// buildWindowsUpdateScript creates a batch file that keeps trying to copy the downloaded version
// over the running one, since Windows refuses the copy until the app has fully closed.
// If every attempt fails, the downloaded file and the script are left in place and a report is
// written so the app can tell the user what happened the next time it starts.
func buildWindowsUpdateScript(opts windowsUpdateScript) string {
	escape := func(value string) string { return strings.ReplaceAll(value, "%", "%%") }

	relaunch := ""
	if opts.Relaunch {
		relaunch = `start "" "%TARGET%"` + "\r\n"
	}

	lines := []string{
		"@echo off",
		"setlocal",
		`set "SOURCE=` + escape(opts.NewExePath) + `"`,
		`set "TARGET=` + escape(opts.TargetPath) + `"`,
		`set "FAILURE=` + escape(opts.FailurePath) + `"`,
		"set /a ATTEMPT=0",
		":retry",
		`copy /Y "%SOURCE%" "%TARGET%" > NUL 2>&1`,
		"if not errorlevel 1 goto success",
		"set /a ATTEMPT+=1",
		"if %ATTEMPT% GEQ " + strconv.Itoa(opts.Attempts) + " goto failed",
		"ping -n 2 127.0.0.1 > NUL",
		"goto retry",
		":success",
		`del "%SOURCE%" > NUL 2>&1`,
		relaunch + `(goto) 2>nul & del "%~f0"`,
		":failed",
		`> "%FAILURE%" echo The update could not replace "%TARGET%" after %ATTEMPT% attempts. The downloaded version was kept at "%SOURCE%" and can be copied over manually.`,
		relaunch + "exit /b 1",
	}
	return strings.Join(lines, "\r\n") + "\r\n"
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
