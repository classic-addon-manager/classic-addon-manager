package main

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/app"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/logger"
	"encoding/json"

	"embed"
	"flag"
	"os"

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
		OnStartup:        a.Startup,
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
