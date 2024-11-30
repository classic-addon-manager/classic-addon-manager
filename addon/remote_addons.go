package addon

import (
	"ClassicAddonManager/api"
	"ClassicAddonManager/config"
	"ClassicAddonManager/file"
	"ClassicAddonManager/logger"
	"ClassicAddonManager/util"
	"encoding/json"
	"errors"
	"io"
	"os"
	"path/filepath"
	"time"
)

type AddonManifest struct {
	Name        string   `json:"name"`
	Description string   `json:"description"`
	Author      string   `json:"author"`
	Repo        string   `json:"repo"`
	Branch      string   `json:"branch"`
	Tags        []string `json:"tags"`
}

func InstallAddon(manifest AddonManifest) (bool, error) {
	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info("addons.txt not found in AAC path, creating it.")
		CreateAddonsTxt()
	}

	logger.Info("Installing addon:" + manifest.Name + " from " + manifest.Repo)

	zipName := manifest.Name + ".zip"
	release, err := api.GetLatestAddonRelease(manifest.Name)
	if err != nil {
		return false, err
	}

	err = util.DownloadFile(release.ZipballUrl, filepath.Join(config.GetCacheDir(), zipName))
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

	AddManagedAddon(manifest, release)

	logger.Info(manifest.Name + " installed successfully")
	return true, nil
}

// GetAddonManifest https://github.com/classic-addon-manager/addons/releases/latest/download/addons.json
func GetAddonManifest(ignoreCache bool) []AddonManifest {
	cacheDir := config.GetCacheDir()
	manifestPath := filepath.Join(cacheDir, "addons.json")
	// Check if manifest exists
	if !file.FileExists(manifestPath) {
		err := downloadManifest(manifestPath)
		if err != nil {
			return []AddonManifest{} // TODO: handle error, should show something to the frontend

		}
	}

	// Check if manifest is outdated
	t, err := file.GetModifiedTime(manifestPath)
	if err != nil {
		return []AddonManifest{} // TODO: handle error, should show something to the frontend
	}

	//fmt.Println("Time since last manifest download:", time.Since(t))
	//fmt.Println("Target time:", time.Minute*60)
	// If more than 60 minutes old, download new manifest
	if ignoreCache || time.Since(t) > time.Minute*60 {
		err = downloadManifest(manifestPath)
		if err != nil {
			return []AddonManifest{} // TODO: handle error, should show something to the frontend
		}
	}

	manifests, err := parseAddonManifest(manifestPath)
	if err != nil {
		return []AddonManifest{} // TODO: handle error, should show something to the frontend
	}

	return manifests
}

func parseAddonManifest(filePath string) ([]AddonManifest, error) {
	f, err := os.Open(filePath)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	byteValue, err := io.ReadAll(f)
	if err != nil {
		return nil, err
	}

	var manifests []AddonManifest
	err = json.Unmarshal(byteValue, &manifests)
	if err != nil {
		return nil, err
	}

	return manifests, nil
}

func downloadManifest(manifestPath string) error {
	err := util.DownloadFile(
		"https://github.com/classic-addon-manager/addons/releases/latest/download/addons.json",
		manifestPath,
	)
	if err != nil {
		return err
	}
	return nil
}

func CheckForUpdate() {

}
