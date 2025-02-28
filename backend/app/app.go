package app

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/github"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"context"
	_ "embed"
	"encoding/json"
	"path/filepath"

	"github.com/sqweek/dialog"
	"github.com/wailsapp/wails/v2/pkg/runtime"
)

// App struct
type App struct {
	Ctx             context.Context
	installedAddons []addon.Addon
	addonManifest   []addon.AddonManifest
}

// NewApp creates a new App application struct
func NewApp(version string) *App {
	shared.Version = version
	return &App{}
}

// Startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) Startup(ctx context.Context) {
	a.Ctx = ctx

	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		addon.CreateAddonsTxt()
	}

	if !file.FileExists(filepath.Join(config.GetDataDir(), "managed_addons.json")) {
		jsonData, err := json.Marshal([]addon.Addon{})
		if err != nil {
			logger.Error("Error creating managed_addons.json:", err)
			return
		}
		err = file.WriteJSON(filepath.Join(config.GetDataDir(), "managed_addons.json"), jsonData)
		if err != nil {
			dialog.Message("Error writing managed_addons.json: %s", err).Title("Classic Addon Manager Error").Error()
			logger.Error("Error writing managed_addons.json:", err)
		}
	}

	err := addon.LoadManagedAddonsFile()
	if err != nil {
		logger.Error("Error loading managed_addons.json:", err)
	}
}

func (a *App) GetVersion() string {
	return shared.Version
}

func (a *App) GetConfigString(key string) string {
	return config.GetString(key)
}

func (a *App) SetConfigString(key string, value string) {
	config.SetString(key, value)
}

func (a *App) GetConfigBool(key string) bool {
	return config.GetBool(key)
}

func (a *App) SetConfigBool(key string, value bool) {
	config.SetBool(key, value)
}

func (a *App) SelectDirectory() string {
	dir, err := runtime.OpenDirectoryDialog(a.Ctx, runtime.OpenDialogOptions{
		Title: "Select Directory",
	})
	if err != nil {
		logger.Error("Error selecting directory:", err)
		return ""
	}
	return dir
}

func (a *App) GetAddonManifests() []addon.AddonManifest {
	return addon.GetAddonManifest()
}

func (a *App) GetReleases(repo string) ([]github.GithubRelease, error) {
	releases, err := github.GetReleases(repo)
	if err != nil {
		logger.Error("Error getting releases:", err)
		return []github.GithubRelease{}, err
	}
	return releases, nil
}
