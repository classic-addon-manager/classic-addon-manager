package main

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/nativeui"
	"ClassicAddonManager/backend/services"
	"embed"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"net/url"
	"os"
	"path/filepath"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

//go:embed all:frontend/dist
var assets embed.FS

const (
	windowMinWidth  = 985
	windowMinHeight = 640
	fatalErrorTitle = "Classic Addon Manager Error"
	deeplinkScheme  = "classicaddonmanager"
)

func main() {
	if nativeui.HandleNativeErrorArgs(os.Args[1:]) {
		os.Exit(0)
	}

	addonUpdateMode := flag.Bool("check-updates", false, "Run in headless mode to check for addon updates")
	flag.Parse()
	if *addonUpdateMode {
		logger.UseHeadlessLog()
	}
	defer logger.Sync()

	err := config.LoadConfig()
	if err != nil {
		if !errors.Is(err, config.ErrAACNotDetected) {
			if *addonUpdateMode {
				logger.Error("Error loading config:", err)
				logger.Sync()
				os.Exit(1)
			}
			nativeui.ShowFatalError(fatalErrorTitle, err.Error())
			logger.Sync()
			os.Exit(1)
		}

		if *addonUpdateMode {
			logger.Error("Error loading config:", err)
			logger.Sync()
			os.Exit(1)
		}
		nativeui.ShowFatalError(fatalErrorTitle, err.Error()+". Open Settings, disable automatic path detection, and choose the ArcheAge Classic Documents path.")
	}

	aacDir, aacErr := config.GetAACDir()
	var addonsTxtPath string
	if aacErr != nil {
		if *addonUpdateMode {
			logger.Error("Cannot check for updates", aacErr)
			logger.Sync()
			os.Exit(1)
		}
		if err == nil {
			nativeui.ShowFatalError(fatalErrorTitle, aacErr.Error())
		}
	} else {
		addonsTxtPath = filepath.Join(aacDir, "Addon", "addons.txt")
		if !file.FileExists(addonsTxtPath) {
			logger.Info(fmt.Sprintf("addons.txt not found, creating it. Attempted path: %s", addonsTxtPath))
			if err := addon.CreateAddonsTxt(); err != nil {
				logger.Error("Error creating addons.txt:", err)
				if *addonUpdateMode {
					logger.Sync()
					os.Exit(1)
				}
				nativeui.ShowFatalError(fatalErrorTitle, fmt.Sprintf("Error occurred while creating addons.txt: %s. Open Settings and verify the ArcheAge Classic path.", err))
			}
		}
	}

	if *addonUpdateMode {
		updates, err := addon.CheckForUpdates()
		if updates == nil {
			// Total failure: leave the previous notification in place.
			logger.Error("Error checking for addon updates:", err)
			logger.Sync()
			os.Exit(1)
		}
		generationErr := addon.GenerateUpdateAddonLua(updates)
		if generationErr != nil {
			logger.Error("Error generating addon update notification:", generationErr)
			logger.Sync()
			os.Exit(1)
		}
		if err != nil {
			logger.Error("Error checking for addon updates:", err)
			logger.Sync()
			os.Exit(1)
		}
		logger.Sync()
		os.Exit(0)
	}

	if !application.System.IsServer() {
		// Check if required webview dependency is installed (Windows only)
		checkWebView2Installation()

		// Check if another instance is running
		if checkForRunningInstance() {
			logger.Sync()
			os.Exit(0)
		}
	}

	loadPersistedState(func(message string) { nativeui.ShowFatalError(fatalErrorTitle, message) })

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

	if !application.System.IsServer() {
		a.Event.OnApplicationEvent(events.Common.ApplicationStarted, func(event *application.ApplicationEvent) {
			_ = event.Context()
			registerDeeplink()
			go startIPCServer(a)
		})
	}

	mainWindow := a.Window.NewWithOptions(application.WebviewWindowOptions{
		Title:            "Classic Addon Manager",
		Name:             "main",
		Width:            windowMinWidth,
		Height:           windowMinHeight,
		MinWidth:         windowMinWidth,
		MinHeight:        windowMinHeight,
		DisableResize:    false,
		Frameless:        true,
		BackgroundColour: application.NewRGBA(10, 10, 10, 1),
		BackgroundType:   application.BackgroundTypeSolid,
		EnableFileDrop:   true,
	})
	// Re-apply after restore/unmaximise: some platforms drop MinWidth/MinHeight.
	// Do not call SetMinSize before a.Run() — beta.5 panics when app.impl is nil.
	reapplyMinSize := func(_ *application.WindowEvent) {
		mainWindow.SetMinSize(windowMinWidth, windowMinHeight)
	}
	mainWindow.OnWindowEvent(events.Common.WindowDidResize, reapplyMinSize)
	mainWindow.OnWindowEvent(events.Common.WindowUnMaximise, reapplyMinSize)
	mainWindow.OnWindowEvent(events.Common.WindowRestore, reapplyMinSize)

	err = a.Run()

	if err != nil {
		logger.Error("Error running application:", err)
	}
}

// loadPersistedState must run before a.Run() so the frontend's first service
// calls see the loaded auth session and managed addons.
func loadPersistedState(showError func(message string)) {
	dataDir, err := config.GetDataDir()
	if err != nil {
		logger.Error("Cannot access the data directory:", err)
		showError(fmt.Sprintf("Cannot access the data directory: %s", err))
		return
	}

	auth.LoadFromDisk()

	managedAddonsPath := filepath.Join(dataDir, "managed_addons.json")
	if !file.FileExists(managedAddonsPath) {
		jsonData, err := json.Marshal(addon.ManagedAddonsFile{
			Version: addon.ManagedAddonsFileVersion,
			Addons:  []addon.Addon{},
		})
		if err != nil {
			logger.Error("Error creating managed_addons.json:", err)
			return
		}
		err = file.WriteJSON(managedAddonsPath, jsonData)
		if err != nil {
			showError(fmt.Sprintf("Error writing managed_addons.json: %s", err))
			logger.Error("Error writing managed_addons.json:", err)
		}
	}

	err = addon.LoadManagedAddonsFile()
	if err != nil {
		logger.Error("Error loading managed_addons.json:", err)
	}
}

// isAuthDeeplink reports whether raw is a classicaddonmanager://auth deeplink.
func isAuthDeeplink(raw string) bool {
	u, err := url.Parse(raw)
	return err == nil && u.Scheme == deeplinkScheme && u.Host == "auth"
}
