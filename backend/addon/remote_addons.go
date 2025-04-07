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

func InstallAddon(manifest shared.AddonManifest) (bool, error) {
	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info("addons.txt not found in AAC path, creating it.")
		CreateAddonsTxt()
	}

	logger.Info("Installing addon:" + manifest.Name + " from " + manifest.Repo)

	zipName := manifest.Name + ".zip"

	err := util.DownloadFile(api.ApiURL+fmt.Sprintf("/addon/%s/download", manifest.Name), filepath.Join(config.GetCacheDir(), zipName))
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

	ok := AddToAddonsTxt(manifest.Name)
	if !ok {
		return false, nil
	}

	release, err := api.GetLatestAddonRelease(manifest.Name)
	if err != nil {
		return false, err
	}

	AddManagedAddon(manifest, release)

	logger.Info(manifest.Name + " installed successfully")
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
