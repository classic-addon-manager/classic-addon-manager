package updater

import (
	"ClassicAddonManager/backend/logger"
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
)

// ErrSelfUpdateUnsupported is returned when the current installation cannot replace itself.
var ErrSelfUpdateUnsupported = errors.New("automatic updates require Windows or a Linux AppImage, please update this installation manually")

// Swapped out in tests so both the supported and unsupported paths can be exercised on any machine.
var selfUpdateOS = runtime.GOOS

const (
	updateReplaceAttempts = 30
	updateFailureFileName = "update-failed.txt"
)

func updateTempDir() string {
	return filepath.Join(os.TempDir(), "ClassicAddonManager")
}

// SelfUpdateSupported tells the frontend whether this installation can update itself.
func SelfUpdateSupported() bool {
	if selfUpdateOS == "linux" {
		_, err := appImageTarget()
		return err == nil
	}
	return selfUpdateOS == "windows"
}

// ConsumeUpdateFailure returns the report left behind by a failed update, if any, and removes it
// so it is only shown once.
func ConsumeUpdateFailure() (string, error) {
	path, err := updateFailurePath()
	if err != nil {
		return "", err
	}
	return consumeUpdateFailure(path)
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

func SelfUpdate(updateURL string, checksum string) error {
	if selfUpdateOS == "linux" {
		return selfUpdateAppImage(updateURL, checksum)
	}

	if !SelfUpdateSupported() {
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
