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
	"net"
	"net/url"
	"os"
	"path/filepath"
	"time"

	"github.com/Microsoft/go-winio"
	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
	"golang.org/x/sys/windows/registry"

	"github.com/sqweek/dialog"
)

//go:embed all:frontend/dist
var assets embed.FS

const pipeName = `\\.\pipe\ClassicAddonManagerPipe`

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

	// Check if another instance is running
	timeout := 2 * time.Second
	conn, err := winio.DialPipe(pipeName, &timeout)
	if err == nil {
		// Send deeplink to the existing instance
		if len(os.Args) > 1 {
			_, _ = fmt.Fprintln(conn, os.Args[1])
		}
		_ = conn.Close()
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
		Services: []application.Service{
			application.NewService(&services.LocalAddonService{}),
			application.NewService(&services.ApplicationService{}),
			application.NewService(&services.RemoteAddonService{}),
		},
	})

	a.OnApplicationEvent(events.Common.ApplicationStarted, func(event *application.ApplicationEvent) {
		_ = event.Context()
		writeDeeplink()
		go startPipeServer(a)
		startup()
	})

	a.NewWebviewWindowWithOptions(application.WebviewWindowOptions{
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

func startPipeServer(a *application.App) {
	// Check if the pipe already exists and remove it
	if _, err := os.Stat(pipeName); err == nil {
		_ = os.Remove(pipeName)
	}

	// Create a permissive security descriptor
	// D:(A;;GA;;;WD) = Allow Everyone Generic All access
	// (A;;GA;;;IU) = Allow Interactive Users Generic All access
	pipeConfig := &winio.PipeConfig{
		SecurityDescriptor: "D:(A;;GA;;;WD)(A;;GA;;;IU)",
	}

	listener, err := winio.ListenPipe(pipeName, pipeConfig)
	if err != nil {
		logger.Error("Error starting pipe server:", err)
		return
	}
	defer listener.Close()
	defer os.Remove(pipeName)

	for {
		conn, err := listener.Accept()
		if err != nil {
			logger.Error("Error accepting connection:", err)
			continue
		}

		go handlePipeConnection(conn, a)
	}
}

func handlePipeConnection(conn net.Conn, a *application.App) {
	defer conn.Close()

	var deeplink string
	if _, err := fmt.Fscanln(conn, &deeplink); err != nil {
		logger.Error("Error reading from pipe:", err)
		return
	}

	// Handle the deeplink in the existing instance
	parsedURL, err := url.Parse(deeplink)
	if err != nil {
		logger.Error("Failed to parse deeplink URL:", err)
		return
	}

	if parsedURL.Host == "auth" {
		token := parsedURL.Query().Get("t")
		if token != "" {
			logger.Info("Received authentication token")
			mainWindow := a.GetWindowByName("main")
			if mainWindow.IsMinimised() {
				mainWindow.UnMinimise()
			} else {
				mainWindow.Minimise()
				mainWindow.UnMinimise()
				mainWindow.Focus()
			}
			mainWindow.EmitEvent("authTokenReceived", token)
		} else {
			logger.Warn("No token found in deeplink URL")
		}
	}
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

	exePath, err := os.Executable()
	if err != nil {
		logger.Error("Failed to get executable path: %s", err)
		return
	}
	// Sets current executable as the handler for the protocol
	command := `"` + exePath + `" "%1"`
	if err := shellKey.SetStringValue("", command); err != nil {
		logger.Error("Failed to set registry value: %s", err)
		return
	}

	logger.Info("Deeplink key created successfully")
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
