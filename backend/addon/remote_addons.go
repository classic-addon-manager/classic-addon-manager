package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/util"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
)

func InstallAddon(manifest shared.AddonManifest, version string) (bool, error) {
	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info("addons.txt not found in AAC path, creating it.")
		CreateAddonsTxt()
	}

	logger.Info("Installing addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	zipName := manifest.Name + ".zip"

	var url string
	if version == "" || version == "latest" {
		url = fmt.Sprintf("/addon/%s/download", manifest.Name)
	} else {
		url = fmt.Sprintf("/addon/%s/download?version=%s", manifest.Name, version)
	}

	err := util.DownloadFile(api.ApiURL+url, filepath.Join(config.GetCacheDir(), zipName))
	if err != nil {
		return false, err
	}

	if file.FileExists(filepath.Join(config.GetCacheDir(), manifest.Name)) {
		// Remove old addon release so we don't copy possibly removed files over
		err = os.RemoveAll(filepath.Join(config.GetCacheDir(), manifest.Name))
		if err != nil {
			logger.Error("Error removing old addon release:", err)
			return false, err
		}
	}

	err = util.ExtractAddonRelease(zipName, manifest.Name)
	if err != nil {
		logger.Error("Error extracting addon release:", err)
		return false, err
	}

	_ = os.Remove(filepath.Join(config.GetCacheDir(), zipName))

	if !util.MoveAddonRelease(manifest.Name) {
		logger.Error(manifest.Name+" - Error moving addon release", errors.New("error moving addon release"))
		return false, nil
	}

	if err := AddToAddonsTxt(manifest.Name); err != nil {
		return false, err
	}

	if version == "" {
		version = "latest"
	}

	release, err := api.GetAddonRelease(manifest.Name, version)
	if err != nil {
		return false, err
	}

	AddManagedAddon(manifest, release)

	logger.Info(manifest.Name + " installed successfully")
	return true, nil
}

// UpdateAddon updates an existing addon by replacing all files except the persistent .data folder.
func UpdateAddon(manifest shared.AddonManifest, version string) (bool, error) {
	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info("addons.txt not found in AAC path, creating it.")
		CreateAddonsTxt()
	}

	logger.Info("Updating addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	zipName := manifest.Name + ".zip"

	var url string
	if version == "" || version == "latest" {
		url = fmt.Sprintf("/addon/%s/download", manifest.Name)
	} else {
		url = fmt.Sprintf("/addon/%s/download?version=%s", manifest.Name, version)
	}

	// Download the release zip into cache
	err := util.DownloadFile(api.ApiURL+url, filepath.Join(config.GetCacheDir(), zipName))
	if err != nil {
		return false, err
	}

	// Clear any previous extracted release in cache
	cacheExtractDir := filepath.Join(config.GetCacheDir(), manifest.Name)
	if file.FileExists(cacheExtractDir) {
		if err = os.RemoveAll(cacheExtractDir); err != nil {
			logger.Error("Error removing old extracted addon release:", err)
			return false, err
		}
	}

	// Extract the new release into cache
	if err = util.ExtractAddonRelease(zipName, manifest.Name); err != nil {
		logger.Error("Error extracting addon release:", err)
		return false, err
	}
	_ = os.Remove(filepath.Join(config.GetCacheDir(), zipName))

	// Find the root directory of the extracted release (first directory inside cacheExtractDir)
	entries, err := os.ReadDir(cacheExtractDir)
	if err != nil {
		logger.Error("Error reading extracted release directory:", err)
		return false, err
	}

	var rootDir string
	for _, e := range entries {
		if e.IsDir() {
			rootDir = e.Name()
			break
		}
	}
	if rootDir == "" {
		logger.Error(manifest.Name+" - No root directory found in addon release, aborting", errors.New("no root directory found"))
		return false, nil
	}

	destAddonDir := filepath.Join(config.GetAddonDir(), manifest.Name)

	// Ensure destination exists
	if err := os.MkdirAll(destAddonDir, os.ModePerm); err != nil {
		logger.Error("Error creating destination addon directory:", err)
		return false, err
	}

	// Remove everything in destination except the .data folder
	destEntries, err := os.ReadDir(destAddonDir)
	if err != nil {
		logger.Error("Error reading destination addon directory:", err)
		return false, err
	}
	for _, de := range destEntries {
		name := de.Name()
		if name == ".data" {
			continue
		}
		removePath := filepath.Join(destAddonDir, name)
		if err := os.RemoveAll(removePath); err != nil {
			logger.Error("Error removing path during update: ", err)
			return false, err
		}
	}

	// Copy new release contents into destination, skipping .data to preserve it
	srcRoot := filepath.Join(cacheExtractDir, rootDir)
	srcEntries, err := os.ReadDir(srcRoot)
	if err != nil {
		logger.Error("Error reading source release root:", err)
		return false, err
	}
	for _, se := range srcEntries {
		name := se.Name()
		if name == ".data" {
			// Do not copy .data from the release, keep existing persistent data
			continue
		}
		srcPath := filepath.Join(srcRoot, name)
		destPath := filepath.Join(destAddonDir, name)
		if se.IsDir() {
			if err := file.CopyDir(srcPath, destPath); err != nil {
				logger.Error("Error copying updated addon directory:", err)
				return false, err
			}
		} else {
			// Ensure destination directory exists then copy file
			if mkErr := os.MkdirAll(filepath.Dir(destPath), os.ModePerm); mkErr != nil {
				logger.Error("Error creating destination path:", mkErr)
				return false, mkErr
			}
			if err := file.MoveFile(srcPath, destPath); err != nil {
				logger.Error("Error copying updated addon file:", err)
				return false, err
			}
		}
	}

	// Cleanup extracted cache
	_ = os.RemoveAll(cacheExtractDir)

	if err := AddToAddonsTxt(manifest.Name); err != nil {
		return false, err
	}

	// Update managed addon metadata
	if version == "" {
		version = "latest"
	}
	release, err := api.GetAddonRelease(manifest.Name, version)
	if err != nil {
		return false, err
	}
	AddManagedAddon(manifest, release)

	logger.Info(manifest.Name + " updated successfully")
	return true, nil
}

// GetAddonManifest https://aac.gaijin.dev/addons
func GetAddonManifest() []shared.AddonManifest {
	req, err := http.NewRequest("GET", "https://aac.gaijin.dev/addons", nil)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []shared.AddonManifest{}
	}

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []shared.AddonManifest{}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		logger.Error("GetAddonManifest Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return []shared.AddonManifest{}
	}

	var manifests []shared.AddonManifest
	if err := json.NewDecoder(resp.Body).Decode(&manifests); err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []shared.AddonManifest{}
	}

	logger.Info("Retrieved " + strconv.Itoa(len(manifests)) + " addon manifests from remote source")

	return manifests
}
