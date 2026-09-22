package util

import (
	"ClassicAddonManager/backend/config"
	"errors"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/spf13/viper"
)

// viper state is process-global: these tests must not run in parallel.
func setupAddonDirs(t *testing.T) (addonDir, cacheDir string) {
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
	t.Cleanup(func() {
		viper.Set("general.aacpath", prevValue)
	})

	return config.GetAddonDir(), config.GetCacheDir()
}

func writeTestFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		t.Fatalf("mkdir for %s: %v", path, err)
	}
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

func readTestFile(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}
	return string(data)
}

func dirEntryNames(t *testing.T, dir string) []string {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("read dir %s: %v", dir, err)
	}
	names := make([]string, 0, len(entries))
	for _, e := range entries {
		names = append(names, e.Name())
	}
	return names
}

func TestMoveAddonReleasePreservesExistingData(t *testing.T) {
	addonDir, cacheDir := setupAddonDirs(t)
	dest := filepath.Join(addonDir, "MyAddon")

	writeTestFile(t, filepath.Join(dest, "main.lua"), "old main")
	writeTestFile(t, filepath.Join(dest, "stale.lua"), "stale")
	writeTestFile(t, filepath.Join(dest, ".data", "settings.json"), "user settings")

	src := filepath.Join(cacheDir, "MyAddon", "release-root")
	writeTestFile(t, filepath.Join(src, "main.lua"), "new main")
	writeTestFile(t, filepath.Join(src, "extra.lua"), "extra")
	writeTestFile(t, filepath.Join(src, ".data", "defaults.json"), "release defaults")

	if err := MoveAddonRelease("MyAddon"); err != nil {
		t.Fatalf("MoveAddonRelease: %v", err)
	}

	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "new main" {
		t.Fatalf("main.lua = %q, want %q", got, "new main")
	}
	if got := readTestFile(t, filepath.Join(dest, "extra.lua")); got != "extra" {
		t.Fatalf("extra.lua = %q, want %q", got, "extra")
	}
	if _, err := os.Stat(filepath.Join(dest, "stale.lua")); !os.IsNotExist(err) {
		t.Fatal("stale.lua must be removed by the update")
	}
	if got := readTestFile(t, filepath.Join(dest, ".data", "settings.json")); got != "user settings" {
		t.Fatalf(".data/settings.json = %q, want preserved user data", got)
	}
	if _, err := os.Stat(filepath.Join(dest, ".data", "defaults.json")); !os.IsNotExist(err) {
		t.Fatal("existing .data must win over the release's .data")
	}
	if _, err := os.Stat(filepath.Join(cacheDir, "MyAddon")); !os.IsNotExist(err) {
		t.Fatal("extracted cache directory must be removed after install")
	}
	if names := dirEntryNames(t, addonDir); len(names) != 1 || names[0] != "MyAddon" {
		t.Fatalf("addon dir must contain only the installed addon, got %v", names)
	}
}

func TestMoveAddonReleaseFreshInstallKeepsReleaseData(t *testing.T) {
	addonDir, cacheDir := setupAddonDirs(t)

	src := filepath.Join(cacheDir, "Fresh", "root")
	writeTestFile(t, filepath.Join(src, "main.lua"), "main")
	writeTestFile(t, filepath.Join(src, ".data", "defaults.json"), "defaults")

	if err := MoveAddonRelease("Fresh"); err != nil {
		t.Fatalf("MoveAddonRelease: %v", err)
	}

	dest := filepath.Join(addonDir, "Fresh")
	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "main" {
		t.Fatalf("main.lua = %q, want %q", got, "main")
	}
	if got := readTestFile(t, filepath.Join(dest, ".data", "defaults.json")); got != "defaults" {
		t.Fatalf(".data/defaults.json = %q, want release defaults kept", got)
	}
}

func TestMoveAddonReleaseNoRootDirectoryLeavesDest(t *testing.T) {
	addonDir, cacheDir := setupAddonDirs(t)
	dest := filepath.Join(addonDir, "Flat")

	writeTestFile(t, filepath.Join(dest, "main.lua"), "old main")
	writeTestFile(t, filepath.Join(dest, ".data", "keep.json"), "keep")
	writeTestFile(t, filepath.Join(cacheDir, "Flat", "main.lua"), "new main")

	if err := MoveAddonRelease("Flat"); err == nil {
		t.Fatal("expected error for release with no root directory")
	}

	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "old main" {
		t.Fatalf("main.lua = %q, want untouched original", got)
	}
	if got := readTestFile(t, filepath.Join(dest, ".data", "keep.json")); got != "keep" {
		t.Fatalf(".data/keep.json = %q, want untouched original", got)
	}
}

func TestMoveAddonReleaseNoEligibleRootLeavesDest(t *testing.T) {
	addonDir, cacheDir := setupAddonDirs(t)
	dest := filepath.Join(addonDir, "NoMain")

	writeTestFile(t, filepath.Join(dest, "main.lua"), "old main")
	writeTestFile(t, filepath.Join(dest, ".data", "keep.json"), "keep")
	writeTestFile(t, filepath.Join(cacheDir, "NoMain", "aaa", "extra.lua"), "x")
	writeTestFile(t, filepath.Join(cacheDir, "NoMain", "zzz", "other.lua"), "x")

	if err := MoveAddonRelease("NoMain"); err == nil {
		t.Fatal("expected error when no release root contains main.lua")
	}

	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "old main" {
		t.Fatalf("main.lua = %q, want untouched original", got)
	}
	if got := readTestFile(t, filepath.Join(dest, ".data", "keep.json")); got != "keep" {
		t.Fatalf(".data/keep.json = %q, want untouched original", got)
	}
}

func TestMoveAddonReleaseMissingExtractFails(t *testing.T) {
	setupAddonDirs(t)

	if err := MoveAddonRelease("Ghost"); err == nil {
		t.Fatal("expected error for missing extracted release")
	}
}

func TestMoveAddonReleasePreservesExecutableBit(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("executable bits are not representable on windows")
	}

	addonDir, cacheDir := setupAddonDirs(t)
	dest := filepath.Join(addonDir, "Mode")

	seedRelease := func(helperContent string) {
		src := filepath.Join(cacheDir, "Mode", "root")
		writeTestFile(t, filepath.Join(src, "main.lua"), "main")
		writeTestFile(t, filepath.Join(src, ".data", "defaults.json"), "defaults")
		helper := filepath.Join(src, "helper.sh")
		writeTestFile(t, helper, helperContent)
		if err := os.Chmod(helper, 0755); err != nil {
			t.Fatalf("chmod helper: %v", err)
		}
	}
	assertHelperMode := func() {
		t.Helper()
		info, err := os.Stat(filepath.Join(dest, "helper.sh"))
		if err != nil {
			t.Fatalf("stat installed helper: %v", err)
		}
		if got := info.Mode().Perm(); got != 0755 {
			t.Fatalf("installed helper mode = %o, want 0755", got)
		}
	}

	seedRelease("v1")
	if err := MoveAddonRelease("Mode"); err != nil {
		t.Fatalf("MoveAddonRelease fresh install: %v", err)
	}
	assertHelperMode()

	writeTestFile(t, filepath.Join(dest, ".data", "settings.json"), "user settings")
	seedRelease("v2")
	if err := MoveAddonRelease("Mode"); err != nil {
		t.Fatalf("MoveAddonRelease replacement: %v", err)
	}
	assertHelperMode()
	if got := readTestFile(t, filepath.Join(dest, ".data", "settings.json")); got != "user settings" {
		t.Fatalf(".data/settings.json = %q, want preserved user data", got)
	}
}

func TestReplaceAddonDirCopyFailureLeavesDest(t *testing.T) {
	addonDir, _ := setupAddonDirs(t)
	dest := filepath.Join(addonDir, "Broken")

	writeTestFile(t, filepath.Join(dest, "main.lua"), "old main")
	writeTestFile(t, filepath.Join(dest, ".data", "keep.json"), "keep")

	badSrc := filepath.Join(t.TempDir(), "not-a-dir")
	writeTestFile(t, badSrc, "oops")

	if err := replaceAddonDir(badSrc, dest); err == nil {
		t.Fatal("expected staging copy error")
	}

	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "old main" {
		t.Fatalf("main.lua = %q, want untouched original", got)
	}
	if got := readTestFile(t, filepath.Join(dest, ".data", "keep.json")); got != "keep" {
		t.Fatalf(".data/keep.json = %q, want untouched original", got)
	}
	if names := dirEntryNames(t, addonDir); len(names) != 1 || names[0] != "Broken" {
		t.Fatalf("staging leftovers must be cleaned up, addon dir contains %v", names)
	}
}

func TestReplaceAddonDirNonDirectoryDataFails(t *testing.T) {
	addonDir, _ := setupAddonDirs(t)
	dest := filepath.Join(addonDir, "Conflict")

	writeTestFile(t, filepath.Join(dest, "main.lua"), "old main")
	writeTestFile(t, filepath.Join(dest, ".data"), "not a directory")

	src := filepath.Join(t.TempDir(), "release")
	writeTestFile(t, filepath.Join(src, "main.lua"), "new main")

	if err := replaceAddonDir(src, dest); err == nil {
		t.Fatal("expected refusal to replace non-directory .data")
	}
	if got := readTestFile(t, filepath.Join(dest, ".data")); got != "not a directory" {
		t.Fatalf(".data file = %q, want untouched original", got)
	}
	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "old main" {
		t.Fatalf("main.lua = %q, want untouched original", got)
	}
	if names := dirEntryNames(t, addonDir); len(names) != 1 || names[0] != "Conflict" {
		t.Fatalf("staging leftovers must be cleaned up, addon dir contains %v", names)
	}
}

func TestCarryOverAddonDataStatErrorFails(t *testing.T) {
	dest := filepath.Join(t.TempDir(), "Addon")
	stage := t.TempDir()

	err := carryOverAddonData(dest, stage, func(string) (os.FileInfo, error) {
		return nil, errors.New("denied")
	})
	if err == nil || !strings.Contains(err.Error(), "denied") {
		t.Fatalf("expected stat failure, got %v", err)
	}
}

func TestSwapDirectoriesSecondRenameFailureRestores(t *testing.T) {
	parent := t.TempDir()
	dest := filepath.Join(parent, "Addon")
	stage := filepath.Join(parent, "stage")
	writeTestFile(t, filepath.Join(dest, "main.lua"), "original")
	writeTestFile(t, filepath.Join(stage, "main.lua"), "staged")

	calls := 0
	rename := func(oldpath, newpath string) error {
		calls++
		if calls == 2 {
			return errors.New("rename blocked")
		}
		return os.Rename(oldpath, newpath)
	}

	err := swapDirectories(stage, dest, rename)
	if err == nil || !strings.Contains(err.Error(), "rename blocked") {
		t.Fatalf("expected rename failure, got %v", err)
	}
	if calls != 3 {
		t.Fatalf("rename called %d times, want 3 (aside, stage, restore)", calls)
	}
	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "original" {
		t.Fatalf("dest main.lua = %q, want restored original", got)
	}
	if names := dirEntryNames(t, parent); len(names) != 1 || names[0] != "Addon" {
		t.Fatalf("backup and stage must be cleaned up after restore, parent contains %v", names)
	}
}

func TestSwapDirectoriesRestoreFailureRetainsBackup(t *testing.T) {
	parent := t.TempDir()
	dest := filepath.Join(parent, "Addon")
	stage := filepath.Join(parent, "stage")
	writeTestFile(t, filepath.Join(dest, "main.lua"), "original")
	writeTestFile(t, filepath.Join(stage, "main.lua"), "staged")

	calls := 0
	rename := func(oldpath, newpath string) error {
		calls++
		if calls >= 2 {
			return errors.New("denied")
		}
		return os.Rename(oldpath, newpath)
	}

	err := swapDirectories(stage, dest, rename)
	if err == nil || !strings.Contains(err.Error(), "denied") {
		t.Fatalf("expected combined rename failure, got %v", err)
	}
	if calls != 3 {
		t.Fatalf("rename called %d times, want 3 (aside, stage, failed restore)", calls)
	}
	if _, statErr := os.Stat(dest); !os.IsNotExist(statErr) {
		t.Fatal("dest must stay absent when restore fails")
	}

	var backupNames []string
	for _, name := range dirEntryNames(t, parent) {
		if name != "stage" {
			backupNames = append(backupNames, name)
		}
	}
	if len(backupNames) != 1 {
		t.Fatalf("expected exactly one retained backup dir, got %v", backupNames)
	}
	if got := readTestFile(t, filepath.Join(parent, backupNames[0], "main.lua")); got != "original" {
		t.Fatalf("backup main.lua = %q, want original", got)
	}
}

func TestSwapDirectoriesFreshInstall(t *testing.T) {
	parent := t.TempDir()
	dest := filepath.Join(parent, "Addon")
	stage := filepath.Join(parent, "stage")
	writeTestFile(t, filepath.Join(stage, "main.lua"), "staged")

	calls := 0
	rename := func(oldpath, newpath string) error {
		calls++
		return os.Rename(oldpath, newpath)
	}

	if err := swapDirectories(stage, dest, rename); err != nil {
		t.Fatalf("swapDirectories: %v", err)
	}
	if calls != 1 {
		t.Fatalf("rename called %d times, want 1 (no existing dest to set aside)", calls)
	}
	if got := readTestFile(t, filepath.Join(dest, "main.lua")); got != "staged" {
		t.Fatalf("dest main.lua = %q, want staged", got)
	}
}
