//go:build windows

package main

import (
	"ClassicAddonManager/backend/logger"
	"fmt"
	"net"
	"net/url"
	"os"
	"time"

	"github.com/Microsoft/go-winio"
	"github.com/sqweek/dialog"
	"github.com/wailsapp/wails/v3/pkg/application"
	"golang.org/x/sys/windows/registry"
)

const pipeName = `\\.\\pipe\\ClassicAddonManagerPipe`

func checkWebView2Installation() {
	const webView2RegistryPath = `SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`

	key, err := registry.OpenKey(registry.LOCAL_MACHINE, webView2RegistryPath, registry.READ)
	if err != nil {
		// WebView2 not found in registry
		err := fmt.Errorf("failed to open registry key: %w", err)
		errorMsg := "Microsoft Edge WebView2 Runtime is not installed. Please install it from https://developer.microsoft.com/en-us/microsoft-edge/webview2"
		logger.Error(errorMsg, err)
		dialog.Message("%s", errorMsg).Title("Classic Addon Manager Error").Error()
		os.Exit(1)
	}
	defer key.Close()

	// Try to read the 'pv' value
	_, _, err = key.GetStringValue("pv")
	if err != nil {
		// 'pv' key not found
		err := fmt.Errorf("failed to read 'pv' value: %w", err)
		errorMsg := "Microsoft Edge WebView2 Runtime is not properly installed. The 'pv' registry key is missing."
		logger.Error(errorMsg, err)
		dialog.Message("%s", errorMsg).Title("Classic Addon Manager Error").Error()
		os.Exit(1)
	}
}

func checkForRunningInstance() bool {
	timeout := 2 * time.Second
	conn, err := winio.DialPipe(pipeName, &timeout)
	if err == nil {
		// Send deeplink to the existing instance
		if len(os.Args) > 1 {
			_, _ = fmt.Fprintln(conn, os.Args[1])
		}
		_ = conn.Close()
		return true
	}
	return false
}

func startIPCServer(a *application.App) {
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

		go handleIPCConnection(conn, a)
	}
}

func handleIPCConnection(conn net.Conn, a *application.App) {
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
			mainWindow, exists := a.Window.GetByName("main")
			if !exists {
				logger.Error("Error getting main window:", err)
				return
			}
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

func registerDeeplink() {
	protocol := "classicaddonmanager"
	regPath := `Software\Classes\` + protocol

	key, _, err := registry.CreateKey(registry.CURRENT_USER, regPath, registry.SET_VALUE)
	if err != nil {
		logger.Error("Failed to create registry key", err)
		return
	}
	defer key.Close()

	if err := key.SetStringValue("", "URL:"+protocol); err != nil {
		logger.Error("Failed to set registry value", err)
		return
	}
	if err := key.SetStringValue("URL Protocol", ""); err != nil {
		logger.Error("Failed to set registry value", err)
		return
	}

	shellKey, _, err := registry.CreateKey(key, `shell\open\command`, registry.SET_VALUE)
	if err != nil {
		logger.Error("Failed to create registry key", err)
		return
	}
	defer shellKey.Close()

	exePath, err := os.Executable()
	if err != nil {
		logger.Error("Failed to get executable path", err)
		return
	}
	// Sets current executable as the handler for the protocol
	command := `"` + exePath + `" "%1"`
	if err := shellKey.SetStringValue("", command); err != nil {
		logger.Error("Failed to set registry value", err)
		return
	}

	logger.Info("Deeplink key created successfully")
}
