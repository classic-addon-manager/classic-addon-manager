package app

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/github"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"ClassicAddonManager/backend/util"
	"context"
	_ "embed"
	"encoding/json"
	"github.com/sqweek/dialog"
	"github.com/wailsapp/wails/v2/pkg/runtime"
	"golang.org/x/sys/windows/registry"
	"os"
	"path/filepath"
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

func writeDeeplink() {
	protocol := "classicaddonmanager"
	regPath := `Software\Classes\` + protocol

	key, _, err := registry.CreateKey(registry.CURRENT_USER, regPath, registry.SET_VALUE)
	if err != nil {
		logger.Error("Failed to create registry key: %s", err)
		return
	}
	defer key.Close()

	if err := key.SetStringValue("", "URL:"+protocol); err != nil {
		logger.Error("Failed to set registry value: %s", err)
		return
	}
	if err := key.SetStringValue("URL Protocol", ""); err != nil {
		logger.Error("Failed to set registry value: %s", err)
		return
	}

	shellKey, _, err := registry.CreateKey(key, `shell\open\command`, registry.SET_VALUE)
	if err != nil {
		logger.Error("Failed to create registry key: %s", err)
		return
	}
	defer shellKey.Close()

	// Sets current executable as the handler for the protocol
	command := `"` + os.Args[0] + `" "%1"`
	if err := shellKey.SetStringValue("", command); err != nil {
		logger.Error("Failed to set registry value: %s", err)
		return
	}

	logger.Info("Deeplink key created successfully")
}

// Startup is called when the app starts. The context is saved
// so we can call the runtime methods
func (a *App) Startup(ctx context.Context) {
	a.Ctx = ctx

	writeDeeplink()

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

func (a *App) GetAddOns() []addon.Addon {
	return addon.GetAddons()
}

func (a *App) GetAddonManifest() []addon.AddonManifest {
	return addon.GetAddonManifest()
}

func (a *App) InstallAddon(ad addon.AddonManifest) (bool, error) {
	_, err := addon.InstallAddon(ad)
	if err != nil {
		logger.Error("Error installing addon:", err)
		return false, err
	}

	ok := addon.AddToAddonsTxt(ad.Name)
	if !ok {
		logger.Warn(ad.Name + " - Error adding addon to addons.txt")
	}

	return ok, nil
}

func (a *App) UninstallAddon(name string) bool {
	if addon.IsInstalled(name) == false {
		return false
	}

	ok, err := file.RemoveDir(filepath.Join(config.GetAddonDir(), name))
	if err != nil {
		logger.Error("Error removing addon directory:", err)
		return false
	}
	if !ok {
		return false
	}

	return addon.RemoveManagedAddon(name) && addon.RemoveFromAddonsTxt(name)
}

func (a *App) UnmanageAddon(name string) bool {
	return addon.RemoveManagedAddon(name)
}

func (a *App) IsAddonInstalled(name string) bool {
	return addon.IsInstalled(name)
}

func (a *App) GetAddonManifests() []addon.AddonManifest {
	return addon.GetAddonManifest()
}

func (a *App) GetLatestAddonRelease(name string) (api.Release, error) {
	release, err := api.GetLatestAddonRelease(name)
	if err != nil {
		logger.Error("Error getting latest release:", err)
		return api.Release{}, err
	}
	return release, nil
}

func (a *App) GetReleases(repo string) ([]github.GithubRelease, error) {
	releases, err := github.GetReleases(repo)
	if err != nil {
		logger.Error("Error getting releases:", err)
		return []github.GithubRelease{}, err
	}
	return releases, nil
}

func (a *App) GetLatestApplicationRelease() (api.ApplicationRelease, error) {
	release, err := api.GetLatestApplicationRelease()
	if err != nil {
		logger.Error("Error getting latest application release:", err)
		return api.ApplicationRelease{}, err
	}
	return release, nil
}

func (a *App) ResetAddonSettings() error {
	err := addon.ResetAddonSettings()
	if err != nil {
		logger.Error("Error resetting addon settings:", err)
		return err
	}
	return nil
}

func (a *App) DiagnoseIssues() ([]util.LogParseResult, error) {
	return util.DiagnoseIssues()
}
