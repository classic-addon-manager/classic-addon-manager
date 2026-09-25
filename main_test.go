package main

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/services"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestIsAuthDeeplink(t *testing.T) {
	tests := []struct {
		name string
		raw  string
		want bool
	}{
		{"auth with token", "classicaddonmanager://auth?t=abc", true},
		{"scheme case-insensitive", "ClassicAddonManager://auth?t=abc", true},
		{"auth with trailing slash", "classicaddonmanager://auth/?t=abc", true},
		{"auth without token", "classicaddonmanager://auth", true},
		{"empty", "", false},
		{"flag double dash", "--check-updates", false},
		{"flag single dash", "-check-updates", false},
		{"windows path", `C:\foo\bar.txt`, false},
		{"unix path", "/tmp/file", false},
		{"other scheme", "https://auth?t=abc", false},
		{"other host", "classicaddonmanager://other?t=abc", false},
		{"opaque url", "classicaddonmanager:auth?t=abc", false},
		{"newline injection", "classicaddonmanager://auth?t=abc\n--check-updates", false},
		{"userinfo host spoof", "classicaddonmanager://user@evil/auth", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := isAuthDeeplink(tt.raw); got != tt.want {
				t.Errorf("isAuthDeeplink(%q) = %v, want %v", tt.raw, got, tt.want)
			}
		})
	}
}

// The logger singleton opens app.log under the config dir on first use and
// never closes it, so bind it to the shared temp dir before a test points the
// config dir at its own t.TempDir(), Windows cannot remove a temp dir holding
// an open file (the same workaround logger_test.go uses).
func bindLoggerToSharedDir(t *testing.T) {
	t.Helper()
	t.Setenv("APPDATA", os.TempDir())
	t.Setenv("XDG_CONFIG_HOME", os.TempDir())
	logger.Sync()
}

// useTempDataDir points the user config dir at a fresh t.TempDir() and returns
// the data dir loadPersistedState will use (<dir>/ClassicAddonManager).
func useTempDataDir(t *testing.T) string {
	t.Helper()
	bindLoggerToSharedDir(t)
	dir := t.TempDir()
	t.Setenv("APPDATA", dir)
	t.Setenv("XDG_CONFIG_HOME", dir)
	return filepath.Join(dir, "ClassicAddonManager")
}

func TestLoadPersistedStateBeforeFrontendReads(t *testing.T) {
	dataDir := useTempDataDir(t)
	t.Cleanup(auth.ClearToken)

	if err := auth.SaveToDisk("test-token"); err != nil {
		t.Fatalf("SaveToDisk: %v", err)
	}
	auth.ClearToken()

	managed := addon.ManagedAddonsFile{
		Version: addon.ManagedAddonsFileVersion,
		Addons: []addon.Addon{
			{
				Name:      "TestAddon",
				Alias:     "Test Addon",
				Version:   "1.0.0",
				IsManaged: true,
			},
		},
	}
	data, err := json.Marshal(managed)
	if err != nil {
		t.Fatalf("marshal managed addons: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dataDir, "managed_addons.json"), data, 0644); err != nil {
		t.Fatalf("write managed_addons.json: %v", err)
	}

	loadPersistedState(func(message string) {
		t.Errorf("unexpected showError call: %s", message)
	})

	session := (&services.ApplicationService{}).GetAuthSession()
	if session.Token != "test-token" {
		t.Errorf("GetAuthSession().Token = %q, want %q", session.Token, "test-token")
	}
	if addon.FindLocalAddonByName("TestAddon") == nil {
		t.Error("FindLocalAddonByName(\"TestAddon\") = nil, want loaded managed addon")
	}
}

func TestLoadPersistedStateCreatesManagedAddonsFile(t *testing.T) {
	dataDir := useTempDataDir(t)

	var messages []string
	loadPersistedState(func(message string) {
		messages = append(messages, message)
	})

	if len(messages) != 0 {
		t.Fatalf("showError called with %v, want no calls", messages)
	}
	if _, err := os.Stat(filepath.Join(dataDir, "managed_addons.json")); err != nil {
		t.Fatalf("managed_addons.json missing: %v", err)
	}
}

func TestLoadPersistedStateReportsDataDirError(t *testing.T) {
	bindLoggerToSharedDir(t)
	blocker := filepath.Join(t.TempDir(), "config")
	if err := os.WriteFile(blocker, []byte("x"), 0644); err != nil {
		t.Fatalf("write blocker file: %v", err)
	}
	t.Setenv("APPDATA", blocker)
	t.Setenv("XDG_CONFIG_HOME", blocker)

	var messages []string
	loadPersistedState(func(message string) {
		messages = append(messages, message)
	})

	if len(messages) != 1 {
		t.Fatalf("showError called %d times, want 1: %v", len(messages), messages)
	}
	if !strings.Contains(messages[0], "Cannot access the data directory") {
		t.Fatalf("showError message = %q, want it to contain %q", messages[0], "Cannot access the data directory")
	}
}
