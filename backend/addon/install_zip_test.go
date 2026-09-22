package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"archive/zip"
	"os"
	"path/filepath"
	"strings"
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

	// A zip whose main.lua sits at the archive root fails validation.
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

func TestInstallZipInstallsRootAlongsideMetadataDir(t *testing.T) {
	addonDir := setupInstallZipTest(t)
	dest := filepath.Join(addonDir, "Multi")

	if err := os.MkdirAll(dest, 0755); err != nil {
		t.Fatalf("seed dest: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dest, "main.lua"), []byte("old main"), 0644); err != nil {
		t.Fatalf("seed main.lua: %v", err)
	}
	if err := file.WriteLines(filepath.Join(addonDir, "addons.txt"), []string{"Other"}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}
	if _, err := ReadAddonsTxt(); err != nil {
		t.Fatalf("load addons.txt: %v", err)
	}

	zipPath := filepath.Join(t.TempDir(), "Multi.zip")
	writeTestZip(t, zipPath, map[string]string{
		"__MACOSX/._main.lua": "x",
		"root/main.lua":       "new main",
		"root/extra.lua":      "extra",
	})

	name, err := InstallZip(zipPath)
	if err != nil {
		t.Fatalf("InstallZip: %v", err)
	}
	if name != "Multi" {
		t.Fatalf("InstallZip name = %q, want %q", name, "Multi")
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
	if file.FileExists(filepath.Join(dest, "._main.lua")) {
		t.Fatal("metadata file must not be installed")
	}
	if file.FileExists(filepath.Join(dest, "__MACOSX")) {
		t.Fatal("__MACOSX directory must not be installed")
	}
	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{"Other", "Multi"})
}

func TestInstallZipMultipleReleaseRootsLeavesDest(t *testing.T) {
	addonDir := setupInstallZipTest(t)
	dest := filepath.Join(addonDir, "Ambig")

	if err := os.MkdirAll(dest, 0755); err != nil {
		t.Fatalf("seed dest: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dest, "main.lua"), []byte("old main"), 0644); err != nil {
		t.Fatalf("seed main.lua: %v", err)
	}
	if err := file.WriteLines(filepath.Join(addonDir, "addons.txt"), []string{"Ambig", "Other"}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}

	zipPath := filepath.Join(t.TempDir(), "Ambig.zip")
	writeTestZip(t, zipPath, map[string]string{
		"aaa/main.lua": "a",
		"zzz/main.lua": "z",
	})

	if _, err := InstallZip(zipPath); err == nil {
		t.Fatal("expected error for release with multiple candidate roots")
	}

	data, err := os.ReadFile(filepath.Join(dest, "main.lua"))
	if err != nil {
		t.Fatalf("read main.lua: %v", err)
	}
	if string(data) != "old main" {
		t.Fatalf("main.lua = %q, want untouched original", string(data))
	}
	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{"Ambig", "Other"})
}

func TestInstallZipDotSlashReleaseRoot(t *testing.T) {
	addonDir := setupInstallZipTest(t)
	dest := filepath.Join(addonDir, "Dot")

	zipPath := filepath.Join(t.TempDir(), "Dot.zip")
	writeTestZip(t, zipPath, map[string]string{
		"./release/main.lua":  "new main",
		"./release/extra.lua": "extra",
	})

	name, err := InstallZip(zipPath)
	if err != nil {
		t.Fatalf("InstallZip: %v", err)
	}
	if name != "Dot" {
		t.Fatalf("InstallZip name = %q, want %q", name, "Dot")
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
	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{"Dot"})
}

func TestInstallZipClearsStaleExtraction(t *testing.T) {
	addonDir := setupInstallZipTest(t)
	cacheDir := config.GetCacheDir()
	dest := filepath.Join(addonDir, "Retry")

	stale := filepath.Join(cacheDir, "Retry")
	for rel, content := range map[string]string{
		"old/main.lua":  "stale old root",
		"new/stale.lua": "stale",
	} {
		p := filepath.Join(stale, rel)
		if err := os.MkdirAll(filepath.Dir(p), 0755); err != nil {
			t.Fatalf("seed stale dir: %v", err)
		}
		if err := os.WriteFile(p, []byte(content), 0644); err != nil {
			t.Fatalf("seed stale file: %v", err)
		}
	}
	unrelated := filepath.Join(cacheDir, "Unrelated", "keep.txt")
	if err := os.MkdirAll(filepath.Dir(unrelated), 0755); err != nil {
		t.Fatalf("seed unrelated dir: %v", err)
	}
	if err := os.WriteFile(unrelated, []byte("keep"), 0644); err != nil {
		t.Fatalf("seed unrelated file: %v", err)
	}

	zipPath := filepath.Join(t.TempDir(), "Retry.zip")
	writeTestZip(t, zipPath, map[string]string{"new/main.lua": "new main"})

	name, err := InstallZip(zipPath)
	if err != nil {
		t.Fatalf("InstallZip: %v", err)
	}
	if name != "Retry" {
		t.Fatalf("InstallZip name = %q, want %q", name, "Retry")
	}

	data, err := os.ReadFile(filepath.Join(dest, "main.lua"))
	if err != nil {
		t.Fatalf("read main.lua: %v", err)
	}
	if string(data) != "new main" {
		t.Fatalf("main.lua = %q, want %q", string(data), "new main")
	}
	if file.FileExists(filepath.Join(dest, "stale.lua")) {
		t.Fatal("stale file must not be installed")
	}
	if file.FileExists(filepath.Join(dest, "old")) {
		t.Fatal("stale release root must not be installed")
	}
	if got, err := os.ReadFile(unrelated); err != nil || string(got) != "keep" {
		t.Fatalf("unrelated cache dir must be left alone, got %q err %v", got, err)
	}
	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{"Retry"})
}

func TestInstallZipUnsafeBasenamesRejected(t *testing.T) {
	for _, zipName := range []string{"...zip", "..zip", ".zip", "....zip"} {
		t.Run(zipName, func(t *testing.T) {
			addonDir := setupInstallZipTest(t)
			cacheDir := config.GetCacheDir()

			sentinelParent := filepath.Join(filepath.Dir(cacheDir), "sentinel.txt")
			if err := os.WriteFile(sentinelParent, []byte("sentinel"), 0644); err != nil {
				t.Fatalf("seed parent sentinel: %v", err)
			}
			sentinelCache := filepath.Join(cacheDir, "keep.txt")
			if err := os.WriteFile(sentinelCache, []byte("keep"), 0644); err != nil {
				t.Fatalf("seed cache sentinel: %v", err)
			}

			dest := filepath.Join(addonDir, "Existing")
			if err := os.MkdirAll(dest, 0755); err != nil {
				t.Fatalf("seed dest: %v", err)
			}
			if err := os.WriteFile(filepath.Join(dest, "main.lua"), []byte("old main"), 0644); err != nil {
				t.Fatalf("seed main.lua: %v", err)
			}
			if err := file.WriteLines(filepath.Join(addonDir, "addons.txt"), []string{"Existing"}); err != nil {
				t.Fatalf("seed addons.txt: %v", err)
			}

			zipPath := filepath.Join(t.TempDir(), zipName)
			writeTestZip(t, zipPath, map[string]string{"root/main.lua": "x"})

			if _, err := InstallZip(zipPath); err == nil {
				t.Fatalf("expected error for unsafe zip name %q", zipName)
			}

			if got, err := os.ReadFile(sentinelParent); err != nil || string(got) != "sentinel" {
				t.Fatalf("cache parent sentinel must survive %q, got %q err %v", zipName, got, err)
			}
			if got, err := os.ReadFile(sentinelCache); err != nil || string(got) != "keep" {
				t.Fatalf("cache sentinel must survive %q, got %q err %v", zipName, got, err)
			}
			data, err := os.ReadFile(filepath.Join(dest, "main.lua"))
			if err != nil {
				t.Fatalf("read main.lua: %v", err)
			}
			if string(data) != "old main" {
				t.Fatalf("main.lua = %q, want untouched original", string(data))
			}
			addonName := strings.TrimSuffix(zipName, filepath.Ext(zipName))
			if file.FileExists(filepath.Join(cacheDir, addonName+".zip")) {
				t.Fatalf("zip must not be copied to cache for unsafe name %q", zipName)
			}
			assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{"Existing"})
		})
	}
}

func TestInstallZipNestedOnlyMainLuaRejected(t *testing.T) {
	addonDir := setupInstallZipTest(t)

	zipPath := filepath.Join(t.TempDir(), "Nested.zip")
	writeTestZip(t, zipPath, map[string]string{"root/nested/main.lua": "x"})

	if _, err := InstallZip(zipPath); err == nil {
		t.Fatal("expected validation error for nested-only main.lua")
	}
	if file.FileExists(filepath.Join(addonDir, "addons.txt")) {
		t.Fatal("addons.txt must not be created by a failed install")
	}
}
