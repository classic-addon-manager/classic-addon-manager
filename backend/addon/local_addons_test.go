package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"sync"
	"testing"
)

func TestManagedAddonsConcurrentAccess(t *testing.T) {
	t.Setenv("APPDATA", os.TempDir())
	t.Setenv("XDG_CONFIG_HOME", os.TempDir())
	logger.Info("starting managed addon concurrency test")

	dataDir := t.TempDir()
	t.Setenv("APPDATA", dataDir)
	t.Setenv("XDG_CONFIG_HOME", dataDir)

	localAddonsMu.Lock()
	localAddons = make(map[string]Addon)
	localAddonsMu.Unlock()
	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = nil
		localAddonsMu.Unlock()
	})

	const addonCount = 16
	var wg sync.WaitGroup
	for i := 0; i < addonCount; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			name := fmt.Sprintf("test_addon_%d", i)
			AddManagedAddon(shared.AddonManifest{
				Name:         name,
				Dependencies: []string{"dependency"},
			}, api.Release{})
			_ = FindLocalAddonByName(name)
			_ = localAddonsSnapshot()
			SaveManagedAddonsToDisk()
		}(i)
	}
	wg.Wait()

	for i := 0; i < addonCount; i++ {
		name := fmt.Sprintf("test_addon_%d", i)
		addon := FindLocalAddonByName(name)
		if addon == nil {
			t.Fatalf("expected addon %q to be registered", name)
		}
		if addon.Name != name {
			t.Errorf("FindLocalAddonByName(%q) returned addon named %q", name, addon.Name)
		}
	}

	snapshot := localAddonsSnapshot()
	if len(snapshot) != addonCount {
		t.Fatalf("snapshot contains %d addons, want %d", len(snapshot), addonCount)
	}

	data, err := os.ReadFile(managedAddonsFilePath())
	if err != nil {
		t.Fatalf("read managed_addons.json: %v", err)
	}
	var managedFile ManagedAddonsFile
	if err := json.Unmarshal(data, &managedFile); err != nil {
		t.Fatalf("unmarshal managed_addons.json: %v", err)
	}
	if len(managedFile.Addons) != addonCount {
		t.Fatalf("managed_addons.json contains %d addons, want %d", len(managedFile.Addons), addonCount)
	}
}

func TestLoadManagedAddonsFile_MigrationManifestFetchFails(t *testing.T) {
	fetchErr := errors.New("manifest fetch failed")
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		return nil, fetchErr
	})
	logger.Info("starting managed addon migration failure test")

	dataDir := t.TempDir()
	t.Setenv("APPDATA", dataDir)
	t.Setenv("XDG_CONFIG_HOME", dataDir)

	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = nil
		localAddonsMu.Unlock()
	})

	legacy := `[{"name":"legacy_addon","alias":"Legacy Addon","version":"1.0.0","isManaged":true}]`
	fp := managedAddonsFilePath()
	if err := os.WriteFile(fp, []byte(legacy), 0644); err != nil {
		t.Fatalf("write legacy managed_addons.json: %v", err)
	}

	err := LoadManagedAddonsFile()
	if err == nil {
		t.Fatal("expected migration error, got nil")
	}
	if !errors.Is(err, fetchErr) {
		t.Fatalf("expected error wrapping %q, got %v", fetchErr, err)
	}

	addon := FindLocalAddonByName("legacy_addon")
	if addon == nil {
		t.Fatal("expected legacy_addon to be loaded despite migration failure")
	}

	backupPath := filepath.Join(config.GetDataDir(), "managed_addons.json.bak")
	if _, statErr := os.Stat(backupPath); !os.IsNotExist(statErr) {
		t.Fatalf("expected no backup file, stat err: %v", statErr)
	}

	data, readErr := os.ReadFile(fp)
	if readErr != nil {
		t.Fatalf("read managed_addons.json: %v", readErr)
	}
	if string(data) != legacy {
		t.Fatalf("expected file unchanged, got %s", data)
	}
}

func TestLoadManagedAddonsFile_MigrationBackfillsDependencies(t *testing.T) {
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		return []shared.AddonManifest{
			{Name: "legacy_addon", Dependencies: []string{"dep_one", "dep_two"}},
		}, nil
	})
	logger.Info("starting managed addon migration test")

	dataDir := t.TempDir()
	t.Setenv("APPDATA", dataDir)
	t.Setenv("XDG_CONFIG_HOME", dataDir)

	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = nil
		localAddonsMu.Unlock()
	})

	legacy := `[{"name":"legacy_addon","alias":"Legacy Addon","version":"1.0.0","isManaged":true}]`
	fp := managedAddonsFilePath()
	if err := os.WriteFile(fp, []byte(legacy), 0644); err != nil {
		t.Fatalf("write legacy managed_addons.json: %v", err)
	}

	if err := LoadManagedAddonsFile(); err != nil {
		t.Fatalf("expected nil error, got %v", err)
	}

	addon := FindLocalAddonByName("legacy_addon")
	if addon == nil {
		t.Fatal("expected legacy_addon to be loaded")
	}
	if !slices.Equal(addon.Dependencies, []string{"dep_one", "dep_two"}) {
		t.Fatalf("expected backfilled dependencies, got %v", addon.Dependencies)
	}

	backupPath := filepath.Join(config.GetDataDir(), "managed_addons.json.bak")
	backup, err := os.ReadFile(backupPath)
	if err != nil {
		t.Fatalf("read managed_addons.json.bak: %v", err)
	}
	if string(backup) != legacy {
		t.Fatalf("expected backup to hold legacy bytes, got %s", backup)
	}

	data, err := os.ReadFile(fp)
	if err != nil {
		t.Fatalf("read managed_addons.json: %v", err)
	}
	var managedFile ManagedAddonsFile
	if err := json.Unmarshal(data, &managedFile); err != nil {
		t.Fatalf("unmarshal migrated managed_addons.json: %v", err)
	}
	if managedFile.Version != ManagedAddonsFileVersion {
		t.Fatalf("expected version %d, got %d", ManagedAddonsFileVersion, managedFile.Version)
	}
	if len(managedFile.Addons) != 1 || managedFile.Addons[0].Name != "legacy_addon" {
		t.Fatalf("expected migrated addon entry, got %+v", managedFile.Addons)
	}
}
