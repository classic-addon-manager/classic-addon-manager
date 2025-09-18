//go:build linux

package main

import (
	"ClassicAddonManager/backend/logger"
	"fmt"
	"net"
	"net/url"
	"os"
	"path/filepath"

	"github.com/wailsapp/wails/v3/pkg/application"
)

const socketPath = "/tmp/classic-addon-manager.sock"

// checkWebView2Installation is not needed on Linux - webkit2gtk is handled by the system
func checkWebView2Installation() {
	// No-op on Linux - webkit2gtk dependency is handled by package manager
}

func checkForRunningInstance() bool {
	conn, err := net.Dial("unix", socketPath)
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
	// Remove existing socket if it exists
	_ = os.Remove(socketPath)

	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		logger.Error("Error starting Unix socket server:", err)
		return
	}
	defer listener.Close()
	defer os.Remove(socketPath)

	// Set permissions on the socket file
	if err := os.Chmod(socketPath, 0600); err != nil {
		logger.Error("Error setting socket permissions:", err)
	}

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
		logger.Error("Error reading from socket:", err)
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
				logger.Error("Error getting main window", fmt.Errorf("main window not found"))
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
	homeDir, err := os.UserHomeDir()
	if err != nil {
		logger.Error("Failed to get home directory", err)
		return
	}

	// Create ~/.local/share/applications if it doesn't exist
	applicationsDir := filepath.Join(homeDir, ".local", "share", "applications")
	if err := os.MkdirAll(applicationsDir, 0755); err != nil {
		logger.Error("Failed to create applications directory", err)
		return
	}

	// Get executable path
	exePath, err := os.Executable()
	if err != nil {
		logger.Error("Failed to get executable path", err)
		return
	}

	// Create .desktop file content
	desktopContent := fmt.Sprintf(`[Desktop Entry]
Version=1.0
Name=Classic Addon Manager
Comment=An addon manager for ArcheAge Classic
Exec=%s %%u
Terminal=false
Type=Application
Icon=classicaddonmanager
Categories=Utility;
StartupWMClass=classicaddonmanager
MimeType=x-scheme-handler/classicaddonmanager;
`, exePath)

	desktopFile := filepath.Join(applicationsDir, "classic-addon-manager.desktop")

	// Write .desktop file
	if err := os.WriteFile(desktopFile, []byte(desktopContent), 0644); err != nil {
		logger.Error("Failed to write .desktop file", err)
		return
	}

	// Update desktop database (if available)
	if _, err := os.Stat("/usr/bin/update-desktop-database"); err == nil {
		// update-desktop-database is available, use it
		logger.Info("Updating desktop database...")
		// Note: We don't run this automatically as it might require user interaction
		// The user should run: update-desktop-database ~/.local/share/applications
	}

	logger.Info(fmt.Sprintf("Desktop file created successfully at %s", desktopFile))
	logger.Info("You may need to run: update-desktop-database ~/.local/share/applications")
}
