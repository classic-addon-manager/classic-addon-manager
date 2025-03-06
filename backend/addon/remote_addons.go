package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/util"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"time"
)

type AddonManifest struct {
	Name         string    `json:"name"`
	Alias        string    `json:"alias"`
	Dependencies []string  `json:"dependencies"`
	Description  string    `json:"description"`
	Author       string    `json:"author"`
	Repo         string    `json:"repo"`
	Branch       string    `json:"branch"`
	Tags         []string  `json:"tags"`
	Downloads    int       `json:"downloads"`
	LikePercent  *int8     `json:"like_percentage"`
	Kofi         *string   `json:"kofi,omitempty"`
	AddedAt      time.Time `json:"added_at"`
}

func InstallAddon(manifest AddonManifest) (bool, error) {
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
func GetAddonManifest() []AddonManifest {
	req, err := http.NewRequest("GET", "https://aac.gaijin.dev/addons", nil)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []AddonManifest{}
	}

	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []AddonManifest{}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		logger.Error("GetAddonManifest Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return []AddonManifest{}
	}

	var manifests []AddonManifest
	if err := json.NewDecoder(resp.Body).Decode(&manifests); err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return []AddonManifest{}
	}

	logger.Info("Retrieved " + strconv.Itoa(len(manifests)) + " addon manifests from remote source")

	return manifests
}

// GetAddonManifestLocal This function used to be called however it should be transformed into a fallback if the HTTP call fails
func GetAddonManifestLocal(ignoreCache bool) []AddonManifest {
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
