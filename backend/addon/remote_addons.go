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
	ensureAddonsTxtExists()

	logger.Info("Installing addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	if err := downloadAndExtractAddon(manifest, version); err != nil {
		return false, err
	}

	if !util.MoveAddonRelease(manifest.Name) {
		logger.Error(manifest.Name+" - Error moving addon release", errors.New("error moving addon release"))
		return false, nil
	}

	if err := AddToAddonsTxt(manifest.Name); err != nil {
		return false, err
	}

	if err := updateAddonMetadata(manifest, version); err != nil {
		return false, err
	}

	logger.Info(manifest.Name + " installed successfully")
	return true, nil
}

// UpdateAddon updates an existing addon by replacing all files except the persistent .data folder.
func UpdateAddon(manifest shared.AddonManifest, version string) (bool, error) {
	ensureAddonsTxtExists()

	logger.Info("Updating addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	if err := downloadAndExtractAddon(manifest, version); err != nil {
		return false, err
	}

	if err := performUpdateFileOperations(manifest); err != nil {
		return false, err
	}

	if err := AddToAddonsTxt(manifest.Name); err != nil {
		return false, err
	}

	if err := updateAddonMetadata(manifest, version); err != nil {
		return false, err
	}

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

func ensureAddonsTxtExists() {
	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info("addons.txt not found in AAC path, creating it.")
		CreateAddonsTxt()
	}
}

func buildDownloadURL(manifest shared.AddonManifest, version string) string {
	if version == "" || version == "latest" {
		return fmt.Sprintf("/addon/%s/download", manifest.Name)
	}
	return fmt.Sprintf("/addon/%s/download?version=%s", manifest.Name, version)
}

func downloadAndExtractAddon(manifest shared.AddonManifest, version string) error {
	zipName := manifest.Name + ".zip"
	url := buildDownloadURL(manifest, version)

	if err := util.DownloadFile(api.ApiURL+url, filepath.Join(config.GetCacheDir(), zipName)); err != nil {
		return err
	}

	if file.FileExists(filepath.Join(config.GetCacheDir(), manifest.Name)) {
		if err := os.RemoveAll(filepath.Join(config.GetCacheDir(), manifest.Name)); err != nil {
			return err
		}
	}

	if err := util.ExtractAddonRelease(zipName, manifest.Name); err != nil {
		return err
	}

	return os.Remove(filepath.Join(config.GetCacheDir(), zipName))
}

func updateAddonMetadata(manifest shared.AddonManifest, version string) error {
	if version == "" {
		version = "latest"
	}

	release, err := api.GetAddonRelease(manifest.Name, version)
	if err != nil {
		return err
	}

	AddManagedAddon(manifest, release)
	return nil
}

func performUpdateFileOperations(manifest shared.AddonManifest) error {
	cacheExtractDir := filepath.Join(config.GetCacheDir(), manifest.Name)

	// Find the root directory of the extracted release
	entries, err := os.ReadDir(cacheExtractDir)
	if err != nil {
		return err
	}

	var rootDir string
	for _, e := range entries {
		if e.IsDir() {
			rootDir = e.Name()
			break
		}
	}
	if rootDir == "" {
		return errors.New("no root directory found")
	}

	destAddonDir := filepath.Join(config.GetAddonDir(), manifest.Name)

	// Ensure destination exists
	if err := os.MkdirAll(destAddonDir, os.ModePerm); err != nil {
		return err
	}

	// Remove everything in destination except the .data folder
	destEntries, err := os.ReadDir(destAddonDir)
	if err != nil {
		return err
	}
	for _, de := range destEntries {
		name := de.Name()
		if name == ".data" {
			continue
		}
		removePath := filepath.Join(destAddonDir, name)
		if err := os.RemoveAll(removePath); err != nil {
			return err
		}
	}

	// Copy new release contents into destination, skipping .data
	srcRoot := filepath.Join(cacheExtractDir, rootDir)
	srcEntries, err := os.ReadDir(srcRoot)
	if err != nil {
		return err
	}
	for _, se := range srcEntries {
		name := se.Name()
		if name == ".data" {
			continue
		}
		srcPath := filepath.Join(srcRoot, name)
		destPath := filepath.Join(destAddonDir, name)
		if se.IsDir() {
			if err := file.CopyDir(srcPath, destPath); err != nil {
				return err
			}
		} else {
			if mkErr := os.MkdirAll(filepath.Dir(destPath), os.ModePerm); mkErr != nil {
				return mkErr
			}
			if err := file.MoveFile(srcPath, destPath); err != nil {
				return err
			}
		}
	}

	// Cleanup extracted cache
	return os.RemoveAll(cacheExtractDir)
}
