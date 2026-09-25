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
	"slices"
	"strconv"
	"sync"
	"time"
)

func InstallAddon(manifest shared.AddonManifest, version string) (bool, error) {
	if err := ensureAddonsTxtExists(); err != nil {
		return false, err
	}

	logger.Info("Installing addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	release, err := fetchAddonRelease(manifest, version)
	if err != nil {
		return false, err
	}

	if err := downloadAndExtract(manifest, version); err != nil {
		return false, err
	}

	if err := util.MoveAddonRelease(manifest.Name); err != nil {
		logger.Error(manifest.Name+" - Error moving addon release", err)
		return false, err
	}

	if err := AddToAddonsTxt(manifest.Name); err != nil {
		return false, err
	}

	AddManagedAddon(manifest, release)

	logger.Info(manifest.Name + " installed successfully")
	return true, nil
}

// UpdateAddon updates an existing addon by replacing all files, preserving an existing .data folder or creating it from the release if missing.
func UpdateAddon(manifest shared.AddonManifest, version string) (bool, error) {
	if err := ensureAddonsTxtExists(); err != nil {
		return false, err
	}

	logger.Info("Updating addon:" + manifest.Name + " from " + manifest.Repo + " version: " + version)

	release, err := fetchAddonRelease(manifest, version)
	if err != nil {
		return false, err
	}

	if err := downloadAndExtract(manifest, version); err != nil {
		return false, err
	}

	if err := util.MoveAddonRelease(manifest.Name); err != nil {
		logger.Error(manifest.Name+" - Error moving addon release", err)
		return false, err
	}

	if err := AddToAddonsTxt(manifest.Name); err != nil {
		return false, err
	}

	AddManagedAddon(manifest, release)

	logger.Info(manifest.Name + " updated successfully")
	return true, nil
}

const addonManifestTTL = time.Minute

var (
	manifestCacheMu    sync.Mutex
	manifestCache      []shared.AddonManifest // nil = never fetched successfully
	manifestCacheUntil time.Time
	// Tests replace these.
	fetchAddonManifest = fetchAddonManifestRemote
	now                = time.Now
)

type manifestFetch struct {
	done      chan struct{}
	manifests []shared.AddonManifest
	err       error
}

var (
	manifestInflight *manifestFetch
	manifestGen      uint64
)

var (
	getAddonRelease    = api.GetAddonRelease
	downloadAndExtract = downloadAndExtractAddon
)

// GetAddonManifest returns the remote catalog, cached for addonManifestTTL.
// Callers that arrive during a fetch share its result, including failures. A
// failed fetch is not cached, the last good catalog is served instead, the
// error is returned only when no cached catalog exists.
func GetAddonManifest() ([]shared.AddonManifest, error) {
	manifestCacheMu.Lock()
	defer manifestCacheMu.Unlock()
	if manifestCache != nil && now().Before(manifestCacheUntil) {
		return slices.Clone(manifestCache), nil
	}
	f := manifestInflight
	if f == nil {
		f = &manifestFetch{done: make(chan struct{})}
		manifestInflight = f
		gen := manifestGen
		manifestCacheMu.Unlock()
		f.manifests, f.err = fetchAddonManifest()
		manifestCacheMu.Lock()
		if manifestInflight == f {
			manifestInflight = nil
		}
		if f.err == nil {
			if f.manifests == nil {
				f.manifests = []shared.AddonManifest{}
			}
			if gen == manifestGen {
				manifestCache = f.manifests
				manifestCacheUntil = now().Add(addonManifestTTL)
			}
		}
		close(f.done)
	} else {
		manifestCacheMu.Unlock()
		<-f.done
		manifestCacheMu.Lock()
	}
	if f.err != nil {
		if manifestCache != nil {
			logger.Warn("GetAddonManifest: serving last cached catalog")
			return slices.Clone(manifestCache), nil
		}
		return nil, fmt.Errorf("fetch addon manifest: %w", f.err)
	}
	return slices.Clone(f.manifests), nil
}

// InvalidateAddonManifestCache forces the next GetAddonManifest call to fetch again, even when a fetch is already running, and keeps the last good catalog as the error fallback.
func InvalidateAddonManifestCache() {
	manifestCacheMu.Lock()
	defer manifestCacheMu.Unlock()
	manifestGen++
	manifestCacheUntil = time.Time{}
	manifestInflight = nil // later callers start a fresh fetch instead of joining one that began before invalidation
}

func fetchAddonManifestRemote() ([]shared.AddonManifest, error) {
	req, err := api.NewApiRequest(nil, http.MethodGet, "/addons", nil)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return nil, err
	}

	resp, err := api.Client.Do(req)
	if err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		err := errors.New(strconv.Itoa(resp.StatusCode))
		logger.Error("GetAddonManifest Error: Status Code", err)
		return nil, err
	}

	var manifests []shared.AddonManifest
	if err := json.NewDecoder(resp.Body).Decode(&manifests); err != nil {
		logger.Error("GetAddonManifest Error:", err)
		return nil, err
	}

	logger.Info("Retrieved " + strconv.Itoa(len(manifests)) + " addon manifests from remote source")

	return manifests, nil
}

func ensureAddonsTxtExists() error {
	txtPath, err := addonsTxtPath()
	if err != nil {
		logger.Error("Could not resolve addons.txt path:", err)
		return err
	}
	if !file.FileExists(txtPath) {
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
	cacheDir, err := config.GetCacheDir()
	if err != nil {
		return err
	}

	zipName := manifest.Name + ".zip"
	url := buildDownloadURL(manifest, version)

	if err := util.DownloadFile(api.ApiURL+url, filepath.Join(cacheDir, zipName)); err != nil {
		return err
	}

	if file.FileExists(filepath.Join(cacheDir, manifest.Name)) {
		if err := os.RemoveAll(filepath.Join(cacheDir, manifest.Name)); err != nil {
			return err
		}
	}

	if err := util.ExtractAddonRelease(zipName, manifest.Name); err != nil {
		return err
	}

	return os.Remove(filepath.Join(cacheDir, zipName))
}

func fetchAddonRelease(manifest shared.AddonManifest, version string) (api.Release, error) {
	if version == "" {
		version = "latest"
	}
	return getAddonRelease(manifest.Name, version)
}
