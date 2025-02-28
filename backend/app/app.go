package app

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/github"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"context"
	_ "embed"
	"github.com/wailsapp/wails/v2/pkg/runtime"
)

// App struct
type App struct {
	Ctx             context.Context
	installedAddons []addon.Addon
	addonManifest   []addon.AddonManifest
}

func (a *App) GetVersion() string {
	return shared.Version
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

func (a *App) GetReleases(repo string) ([]github.GithubRelease, error) {
	releases, err := github.GetReleases(repo)
	if err != nil {
		logger.Error("Error getting releases:", err)
		return []github.GithubRelease{}, err
	}
	return releases, nil
}
