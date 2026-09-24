package config

import (
	"errors"
	"os"
	"path/filepath"
	"testing"

	"github.com/spf13/viper"
)

// Environment variables and viper state are process-global: these tests must
// not run in parallel.

func TestGetCacheDirResolutionFails(t *testing.T) {
	t.Setenv("LocalAppData", "")
	t.Setenv("XDG_CACHE_HOME", "")
	t.Setenv("HOME", "")

	dir, err := GetCacheDir()
	if err == nil {
		t.Fatal("expected error when user cache directory cannot be resolved")
	}
	if dir != "" {
		t.Fatalf("GetCacheDir = %q, want empty path on error", dir)
	}
}

func TestGetCacheDirCreateFails(t *testing.T) {
	blocker := filepath.Join(t.TempDir(), "cache")
	if err := os.WriteFile(blocker, []byte("x"), 0644); err != nil {
		t.Fatalf("write blocker file: %v", err)
	}
	t.Setenv("LocalAppData", blocker)
	t.Setenv("XDG_CACHE_HOME", blocker)

	dir, err := GetCacheDir()
	if err == nil {
		t.Fatal("expected error when cache directory cannot be created")
	}
	if dir != "" {
		t.Fatalf("GetCacheDir = %q, want empty path on error", dir)
	}
}

func TestGetCacheDirCreatesDirectory(t *testing.T) {
	parent := t.TempDir()
	t.Setenv("LocalAppData", parent)
	t.Setenv("XDG_CACHE_HOME", parent)

	dir, err := GetCacheDir()
	if err != nil {
		t.Fatalf("GetCacheDir: %v", err)
	}
	want := filepath.Join(parent, "ClassicAddonManager")
	if dir != want {
		t.Fatalf("GetCacheDir = %q, want %q", dir, want)
	}
	info, err := os.Stat(dir)
	if err != nil || !info.IsDir() {
		t.Fatalf("cache directory must exist, info = %v, err = %v", info, err)
	}
}

func TestGetDataDirResolutionFails(t *testing.T) {
	t.Setenv("APPDATA", "")
	t.Setenv("XDG_CONFIG_HOME", "")
	t.Setenv("HOME", "")

	dir, err := GetDataDir()
	if err == nil {
		t.Fatal("expected error when user config directory cannot be resolved")
	}
	if dir != "" {
		t.Fatalf("GetDataDir = %q, want empty path on error", dir)
	}
}

func TestGetDataDirCreateFails(t *testing.T) {
	blocker := filepath.Join(t.TempDir(), "config")
	if err := os.WriteFile(blocker, []byte("x"), 0644); err != nil {
		t.Fatalf("write blocker file: %v", err)
	}
	t.Setenv("APPDATA", blocker)
	t.Setenv("XDG_CONFIG_HOME", blocker)

	dir, err := GetDataDir()
	if err == nil {
		t.Fatal("expected error when data directory cannot be created")
	}
	if dir != "" {
		t.Fatalf("GetDataDir = %q, want empty path on error", dir)
	}
}

func TestGetDataDirCreatesDirectory(t *testing.T) {
	parent := t.TempDir()
	t.Setenv("APPDATA", parent)
	t.Setenv("XDG_CONFIG_HOME", parent)

	dir, err := GetDataDir()
	if err != nil {
		t.Fatalf("GetDataDir: %v", err)
	}
	want := filepath.Join(parent, "ClassicAddonManager")
	if dir != want {
		t.Fatalf("GetDataDir = %q, want %q", dir, want)
	}
	info, err := os.Stat(dir)
	if err != nil || !info.IsDir() {
		t.Fatalf("data directory must exist, info = %v, err = %v", info, err)
	}
}

func setAACPath(t *testing.T, value any) {
	t.Helper()
	prev := viper.Get("general.aacpath")
	viper.Set("general.aacpath", value)
	t.Cleanup(func() { viper.Set("general.aacpath", prev) })
}

func TestGetAACDirUnset(t *testing.T) {
	setAACPath(t, nil)

	dir, err := GetAACDir()
	if !errors.Is(err, ErrAACPathNotSet) {
		t.Fatalf("GetAACDir err = %v, want ErrAACPathNotSet", err)
	}
	if dir != "" {
		t.Fatalf("GetAACDir = %q, want empty path on error", dir)
	}
}

func TestGetAACDirRelativePath(t *testing.T) {
	setAACPath(t, "rel/path")

	dir, err := GetAACDir()
	if err == nil {
		t.Fatal("expected error for relative ArcheAge Classic path")
	}
	if dir != "" {
		t.Fatalf("GetAACDir = %q, want empty path on error", dir)
	}
}

func TestGetAACDirAbsolutePath(t *testing.T) {
	want := t.TempDir()
	setAACPath(t, want)

	dir, err := GetAACDir()
	if err != nil {
		t.Fatalf("GetAACDir: %v", err)
	}
	if dir != want {
		t.Fatalf("GetAACDir = %q, want %q", dir, want)
	}
}

func TestGetAddonDirUnset(t *testing.T) {
	setAACPath(t, nil)

	dir, err := GetAddonDir()
	if !errors.Is(err, ErrAACPathNotSet) {
		t.Fatalf("GetAddonDir err = %v, want ErrAACPathNotSet", err)
	}
	if dir != "" {
		t.Fatalf("GetAddonDir = %q, want empty path on error", dir)
	}
}

func TestGetAddonDirAbsolutePath(t *testing.T) {
	aacDir := t.TempDir()
	setAACPath(t, aacDir)

	dir, err := GetAddonDir()
	if err != nil {
		t.Fatalf("GetAddonDir: %v", err)
	}
	want := filepath.Join(aacDir, "Addon")
	if dir != want {
		t.Fatalf("GetAddonDir = %q, want %q", dir, want)
	}
}
