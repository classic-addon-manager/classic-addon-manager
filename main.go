package main

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/app"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/logger"
	"context"
	"embed"
	"encoding/json"
	"flag"
	"fmt"
	"github.com/Microsoft/go-winio"
	"github.com/wailsapp/wails/v2/pkg/runtime"
	"net"
	"net/url"
	"os"
	"time"

	"github.com/sqweek/dialog"
	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/options/windows"
)

//go:embed all:frontend/dist
var assets embed.FS

//go:embed wails.json
var wailsConfigData []byte

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

	type AppInfo struct {
		Info struct {
			ProductVersion string `json:"productVersion"`
		} `json:"info"`
	}

	var w AppInfo
	if err := json.Unmarshal(wailsConfigData, &w); err != nil {
		logger.Error("Error unmarshalling wails.json:", err)
	}

	a := app.NewApp(w.Info.ProductVersion)

	err = wails.Run(&options.App{
		Title:         "Classic Addon Manager",
		Width:         950,
		Height:        600,
		MinHeight:     600,
		MinWidth:      950,
		DisableResize: false,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 27, G: 38, B: 54, A: 1},
		OnStartup: func(ctx context.Context) {
			a.Ctx = ctx
			a.Startup(ctx)
			go startPipeServer(a)
		},
		Bind: []interface{}{
			a,
		},
		Windows: &windows.Options{
			DisablePinchZoom: true,
		},
	})

	if err != nil {
		println("Error:", err.Error())
	}
}

func startPipeServer(a *app.App) {
	// Check if the pipe already exists and remove it
	if _, err := os.Stat(pipeName); err == nil {
		_ = os.Remove(pipeName)
	}

	listener, err := winio.ListenPipe(pipeName, nil)
	if err != nil {
		logger.Error("Error starting pipe server:", err)
		return
	}
	defer listener.Close()

	runtime.EventsOn(a.Ctx, "wails:shutdown", func(optionalData ...interface{}) {
		_ = listener.Close()
		_ = os.Remove(pipeName)
	})

	for {
		conn, err := listener.Accept()
		if err != nil {
			logger.Error("Error accepting connection:", err)
			continue
		}

		go handlePipeConnection(conn, a)
	}
}

func handlePipeConnection(conn net.Conn, a *app.App) {
	defer conn.Close()

	var deeplink string
	_, err := fmt.Fscanln(conn, &deeplink)
	if err != nil {
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
			if runtime.WindowIsMinimised(a.Ctx) {
				runtime.WindowUnminimise(a.Ctx)
			} else {
				runtime.WindowMinimise(a.Ctx)
				runtime.WindowUnminimise(a.Ctx)
			}
			runtime.WindowShow(a.Ctx)
			runtime.EventsEmit(a.Ctx, "authTokenReceived", token)
		} else {
			logger.Warn("No token found in deeplink URL")
		}
	}
}
