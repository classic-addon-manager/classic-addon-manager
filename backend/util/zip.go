package util

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"archive/zip"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
)

func ExtractAddonRelease(src string, dest string) error {
	tmpSrc := filepath.Join(config.GetCacheDir(), src)
	if !file.FileExists(tmpSrc) {
		return fmt.Errorf("file %s does not exist", tmpSrc)
	}

	archive, err := zip.OpenReader(tmpSrc)
	if err != nil {
		return err
	}
	defer archive.Close()

	tmpDest := filepath.Join(config.GetCacheDir(), dest)

	for _, f := range archive.File {
		fPath := filepath.Join(tmpDest, f.Name)
		logger.Info("Extracting: " + fPath)

		if !strings.HasPrefix(fPath, filepath.Clean(tmpDest)+string(os.PathSeparator)) {
			return fmt.Errorf("%s: invalid file path", fPath)
		}

		if f.FileInfo().IsDir() {
			logger.Info("Creating directory: " + fPath)
			err := os.MkdirAll(fPath, os.ModePerm)
			if err != nil {
				return err
			}
			continue
		}

		if err = os.MkdirAll(filepath.Dir(fPath), os.ModePerm); err != nil {
			return err
		}

		if err := extractFile(f, fPath); err != nil {
			return err
		}
	}

	return nil
}

func extractFile(f *zip.File, dest string) error {
	fileInArchive, err := f.Open()
	if err != nil {
		return err
	}
	defer fileInArchive.Close()

	dstFile, err := os.OpenFile(dest, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, f.Mode())
	if err != nil {
		return err
	}
	defer dstFile.Close()

	_, err = io.Copy(dstFile, fileInArchive)
	return err
}

func MoveAddonRelease(addonName string) error {
	src := filepath.Join(config.GetCacheDir(), addonName)
	entries, err := os.ReadDir(src)
	if err != nil {
		return fmt.Errorf("error reading extracted addon release: %w", err)
	}

	var rootDir string

	for _, entry := range entries {
		if entry.IsDir() {
			rootDir = entry.Name()
			break
		}
	}

	if rootDir == "" {
		return errors.New("no root directory found in addon release")
	}

	if err := replaceAddonDir(filepath.Join(src, rootDir), filepath.Join(config.GetAddonDir(), addonName)); err != nil {
		return err
	}

	_ = os.RemoveAll(src)

	return nil
}

// replaceAddonDir installs the extracted release root at dest without ever
// deleting an existing dest first: the release is staged in a sibling temp
// directory (same filesystem, so the final rename is atomic), a surviving
// .data directory is carried over, and only then is the staged copy swapped
// into place. Any failure before the swap leaves dest untouched.
func replaceAddonDir(releaseRoot, dest string) error {
	parent := filepath.Dir(dest)
	if err := os.MkdirAll(parent, os.ModePerm); err != nil {
		return fmt.Errorf("failed to create addon directory: %w", err)
	}

	stage, err := os.MkdirTemp(parent, "."+filepath.Base(dest)+".staging-")
	if err != nil {
		return fmt.Errorf("failed to create staging directory: %w", err)
	}
	// No-op once stage has been renamed into place.
	defer os.RemoveAll(stage)

	if err := file.CopyDir(releaseRoot, stage); err != nil {
		return fmt.Errorf("failed to stage addon release: %w", err)
	}

	if err := carryOverAddonData(dest, stage, os.Stat); err != nil {
		return err
	}

	return swapDirectories(stage, dest, os.Rename)
}

// carryOverAddonData replaces the staged release's .data with a copy of the
// existing install's .data. A missing .data keeps the release's copy, an
// unstat-able or non-directory .data fails the install rather than risking
// the user's data.
func carryOverAddonData(dest, stage string, stat func(string) (os.FileInfo, error)) error {
	info, err := stat(filepath.Join(dest, ".data"))
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return fmt.Errorf("failed to inspect existing .data: %w", err)
	}
	if !info.IsDir() {
		return fmt.Errorf("existing .data at %q is not a directory", filepath.Join(dest, ".data"))
	}

	stagedData := filepath.Join(stage, ".data")
	if err := os.RemoveAll(stagedData); err != nil {
		return fmt.Errorf("failed to replace staged .data: %w", err)
	}
	if err := file.CopyDir(filepath.Join(dest, ".data"), stagedData); err != nil {
		return fmt.Errorf("failed to preserve existing .data: %w", err)
	}
	return nil
}

// swapDirectories moves dest aside to a unique backup sibling, renames stage
// into place, and removes the backup. If the stage rename fails, the backup is
// restored, if the restore also fails the backup is retained and both errors
// are reported.
func swapDirectories(stage, dest string, rename func(oldpath, newpath string) error) error {
	var backup string
	if _, err := os.Stat(dest); err == nil {
		placeholder, err := os.MkdirTemp(filepath.Dir(dest), filepath.Base(dest)+".backup-")
		if err != nil {
			return fmt.Errorf("failed to reserve backup path: %w", err)
		}
		if err := os.Remove(placeholder); err != nil {
			return fmt.Errorf("failed to release backup path: %w", err)
		}
		backup = placeholder

		if err := rename(dest, backup); err != nil {
			return fmt.Errorf("failed to move existing addon aside: %w", err)
		}
	} else if !os.IsNotExist(err) {
		return fmt.Errorf("failed to inspect existing addon directory: %w", err)
	}

	if err := rename(stage, dest); err != nil {
		if backup != "" {
			if restoreErr := rename(backup, dest); restoreErr != nil {
				return fmt.Errorf("failed to move staged addon into place: %w; additionally failed to restore the previous addon from %q: %v", err, backup, restoreErr)
			}
		}
		_ = os.RemoveAll(stage)
		return fmt.Errorf("failed to move staged addon into place: %w", err)
	}

	if backup != "" {
		if err := os.RemoveAll(backup); err != nil {
			logger.Error("Failed to remove addon backup after successful swap:", err)
		}
	}

	return nil
}
