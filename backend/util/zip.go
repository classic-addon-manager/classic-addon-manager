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

func MoveAddonRelease(addonName string) bool {
	src := filepath.Join(config.GetCacheDir(), addonName)
	entries, err := os.ReadDir(src)
	if err != nil {
		logger.Error("Error reading directory:", err)
		return false
	}

	var rootDir string

	for _, entry := range entries {
		if entry.IsDir() {
			rootDir = entry.Name()
			break
		}
	}

	if rootDir == "" {
		logger.Error(addonName+" - No root directory found in addon release, aborting", errors.New("no root directory found"))
		return false
	}

	if file.FileExists(filepath.Join(config.GetAddonDir(), addonName)) {
		err = os.RemoveAll(filepath.Join(config.GetAddonDir(), addonName))
		if err != nil {
			logger.Error("Error removing old addon release:", err)
			return false
		}
	}

	err = file.MoveDir(filepath.Join(src, rootDir), filepath.Join(config.GetAddonDir(), addonName))
	if err != nil {
		logger.Error("Error moving addon release:", err)
		return false
	}

	_ = os.RemoveAll(src)

	return true
}
