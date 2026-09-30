//go:build linux

package updater

import (
	"ClassicAddonManager/backend/logger"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
)

func selfUpdateAppImage(updateURL, checksum string) error {
	update, err := prepareAppImageUpdate(updateURL, checksum)
	if err != nil {
		return err
	}
	if err := startAppImageUpdateHelper(update); err != nil {
		os.RemoveAll(update.StagingDir)
		return err
	}
	logger.Info("AppImage update helper started, exiting application.")
	logger.Sync()
	os.Exit(0)
	return nil
}

func startAppImageUpdateHelper(update appImageUpdate) error {
	failure, err := updateFailurePath()
	if err != nil {
		return err
	}
	if err := os.Remove(failure); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("clearing previous update failure: %w", err)
	}
	script := filepath.Join(update.StagingDir, "update.sh")
	if err := os.WriteFile(script, []byte(appImageUpdateScript), 0600); err != nil {
		return err
	}
	cmd := exec.Command("/bin/sh", script, strconv.Itoa(os.Getpid()), update.TargetPath, update.SourcePath, update.StagingDir, failure)
	appDir := os.Getenv("APPDIR")
	cmd.Env = appImageRelaunchEnvironment(os.Environ(), appDir)
	cmd.Dir = os.Getenv("OWD")
	if cmd.Dir == "" {
		cmd.Dir, err = os.Getwd()
		if err != nil {
			return err
		}
	}
	if appDir != "" && (cmd.Dir == appDir || strings.HasPrefix(cmd.Dir, appDir+string(os.PathSeparator))) {
		cmd.Dir = filepath.Dir(update.TargetPath)
	}
	// Survive termination of the old app's session and do not inherit its stdio.
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("starting AppImage update helper: %w", err)
	}
	cmd.Process.Release()
	return nil
}
