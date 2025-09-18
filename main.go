package main

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/services"
	"embed"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"

	"github.com/sqweek/dialog"
)

//go:embed all:frontend/dist
var assets embed.FS

func main() {
	addonUpdateMode := flag.Bool("check-updates", false, "Run in headless mode to check for addon updates")
	flag.Parse()

	err := config.LoadConfig()
	if err != nil {
		dialog.Message("%s", err.Error()).Title("Classic Addon Manager Error").Error()
		os.Exit(1)
	}

	if *addonUpdateMode {
		addon.GenerateUpdateAddonLua(
			addon.CheckForUpdates(),
		)
		os.Exit(0)
	}

	// Check if required webview dependency is installed (Windows only)
	checkWebView2Installation()

	// Check if another instance is running
	if checkForRunningInstance() {
		os.Exit(0)
	}

	if !file.FileExists(filepath.Join(config.GetAddonDir(), "addons.txt")) {
		logger.Info(fmt.Sprintf("addons.txt not found, creating it. Attempted path: %s", filepath.Join(config.GetAddonDir(), "addons.txt")))
		addon.CreateAddonsTxt()
	}

	a := application.New(application.Options{
		Name: "Classic Addon Manager",
		Assets: application.AssetOptions{
			Handler: application.AssetFileServerFS(assets),
		},
	})

	applicationServices := []application.Service{
		application.NewService(&services.LocalAddonService{}),
		application.NewService(&services.ApplicationService{
			App: a,
		}),
		application.NewService(&services.RemoteAddonService{}),
	}

	for _, service := range applicationServices {
		a.RegisterService(service)
	}

	a.Event.OnApplicationEvent(events.Common.ApplicationStarted, func(event *application.ApplicationEvent) {
		_ = event.Context()
		registerDeeplink()
		go startIPCServer(a)
		startup()
	})

	a.Window.NewWithOptions(application.WebviewWindowOptions{
		Title:            "Classic Addon Manager",
		Name:             "main",
		Width:            985,
		Height:           640,
		MinHeight:        600,
		MinWidth:         950,
		DisableResize:    false,
		BackgroundColour: application.NewRGBA(27, 38, 54, 1),
	})

	err = a.Run()

	if err != nil {
		println("Error:", err.Error())
	}
}

func startup() {
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
