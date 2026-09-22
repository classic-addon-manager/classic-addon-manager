package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"archive/zip"
	"os"
	"path/filepath"
	"testing"

	"github.com/spf13/viper"
)

// viper state is process-global: these tests must not run in parallel.
func setupInstallZipTest(t *testing.T) (addonDir string) {
	t.Helper()

	root := t.TempDir()
	cacheParent := filepath.Join(root, "cache")
	if err := os.MkdirAll(cacheParent, 0755); err != nil {
		t.Fatalf("create cache parent: %v", err)
	}
	prevValue := viper.Get("general.aacpath")
	viper.Set("general.aacpath", filepath.Join(root, "aac"))
	t.Setenv("LocalAppData", cacheParent)
	t.Setenv("XDG_CACHE_HOME", cacheParent)
	setInstalledAddonNames(nil)
	t.Cleanup(func() {
		viper.Set("general.aacpath", prevValue)
		setInstalledAddonNames(nil)
	})

	return config.GetAddonDir()
}

func writeTestZip(t *testing.T, path string, files map[string]string) {
	t.Helper()
	f, err := os.Create(path)
	if err != nil {
		t.Fatalf("create zip %s: %v", path, err)
	}
	w := zip.NewWriter(f)
	for name, content := range files {
		fw, err := w.Create(name)
		if err != nil {
			t.Fatalf("create zip entry %s: %v", name, err)
		}
		if _, err := fw.Write([]byte(content)); err != nil {
			t.Fatalf("write zip entry %s: %v", name, err)
		}
	}
	if err := w.Close(); err != nil {
		t.Fatalf("close zip writer: %v", err)
	}
	if err := f.Close(); err != nil {
		t.Fatalf("close zip file: %v", err)
	}
}

func TestInstallZipOverExistingPreservesData(t *testing.T) {
	addonDir := setupInstallZipTest(t)
	dest := filepath.Join(addonDir, "MyZip")

	if err := os.MkdirAll(filepath.Join(dest, ".data"), 0755); err != nil {
		t.Fatalf("seed .data: %v", err)
	}
	seed := map[string]string{
		"main.lua":            "old main",
		"stale.lua":           "stale",
		".data/settings.json": "user settings",
	}
	for name, content := range seed {
		if err := os.WriteFile(filepath.Join(dest, name), []byte(content), 0644); err != nil {
			t.Fatalf("seed %s: %v", name, err)
		}
	}

	zipPath := filepath.Join(t.TempDir(), "MyZip.zip")
	writeTestZip(t, zipPath, map[string]string{
		"root/main.lua":       "new main",
		"root/extra.lua":      "extra",
		"root/.data/rel.json": "release defaults",
	})

	name, err := InstallZip(zipPath)
	if err != nil {
		t.Fatalf("InstallZip: %v", err)
	}
	if name != "MyZip" {
		t.Fatalf("InstallZip name = %q, want %q", name, "MyZip")
	}

	readDest := func(rel string) string {
		t.Helper()
		data, err := os.ReadFile(filepath.Join(dest, rel))
		if err != nil {
			t.Fatalf("read %s: %v", rel, err)
		}
		return string(data)
	}
	if got := readDest("main.lua"); got != "new main" {
		t.Fatalf("main.lua = %q, want %q", got, "new main")
	}
	if got := readDest("extra.lua"); got != "extra" {
		t.Fatalf("extra.lua = %q, want %q", got, "extra")
	}
	if _, err := os.Stat(filepath.Join(dest, "stale.lua")); !os.IsNotExist(err) {
		t.Fatal("stale.lua must be removed by the reinstall")
	}
	if got := readDest(".data/settings.json"); got != "user settings" {
		t.Fatalf(".data/settings.json = %q, want preserved user data", got)
	}
	if _, err := os.Stat(filepath.Join(dest, ".data", "rel.json")); !os.IsNotExist(err) {
		t.Fatal("existing .data must win over the release's .data")
	}

	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{"MyZip"})
}

func TestInstallZipMalformedReleaseLeavesDest(t *testing.T) {
	addonDir := setupInstallZipTest(t)
	dest := filepath.Join(addonDir, "Flat")

	if err := os.MkdirAll(filepath.Join(dest, ".data"), 0755); err != nil {
		t.Fatalf("seed .data: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dest, ".data", "keep.json"), []byte("keep"), 0644); err != nil {
		t.Fatalf("seed keep.json: %v", err)
	}

	// A zip whose main.lua sits at the archive root passes validation but
	// extracts without a release root directory.
	zipPath := filepath.Join(t.TempDir(), "Flat.zip")
	writeTestZip(t, zipPath, map[string]string{"main.lua": "x"})

	if _, err := InstallZip(zipPath); err == nil {
		t.Fatal("expected error for release with no root directory")
	}

	data, err := os.ReadFile(filepath.Join(dest, ".data", "keep.json"))
	if err != nil {
		t.Fatalf("read keep.json: %v", err)
	}
	if string(data) != "keep" {
		t.Fatalf(".data/keep.json = %q, want untouched original", string(data))
	}
	if file.FileExists(filepath.Join(addonDir, "addons.txt")) {
		t.Fatal("addons.txt must not be created by a failed install")
	}
}
