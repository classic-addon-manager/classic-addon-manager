package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"encoding/json"
	"fmt"
	"os"
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
