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
	if err := ensureAddonsTxtExists(); err != nil {
		return false, err
	}

	logger.Info("Installing addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	if err := downloadAndExtractAddon(manifest, version); err != nil {
		return false, err
	}

	if err := util.MoveAddonRelease(manifest.Name); err != nil {
		logger.Error(manifest.Name+" - Error moving addon release", err)
		return false, err
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

// UpdateAddon updates an existing addon by replacing all files, preserving an existing .data folder or creating it from the release if missing.
func UpdateAddon(manifest shared.AddonManifest, version string) (bool, error) {
	if err := ensureAddonsTxtExists(); err != nil {
		return false, err
	}

	logger.Info("Updating addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	if err := downloadAndExtractAddon(manifest, version); err != nil {
		return false, err
	}

	if err := util.MoveAddonRelease(manifest.Name); err != nil {
		logger.Error(manifest.Name+" - Error moving addon release", err)
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

func GetAddonManifest() []shared.AddonManifest {
	req, err := api.NewApiRequest(nil, http.MethodGet, "/addons", nil)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []shared.AddonManifest{}
	}

	resp, err := api.Client.Do(req)
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

func ensureAddonsTxtExists() error {
	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info("addons.txt not found in AAC path, creating it.")
		if err := CreateAddonsTxt(); err != nil {
			logger.Error("Could not create addons.txt in AAC path:", err)
			return err
		}
	}
	return nil
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
