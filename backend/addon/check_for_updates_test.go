package addon

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
)

func TestGenerateUpdatesLuaQuotesEveryField(t *testing.T) {
	tests := []struct {
		name  string
		addon Addon
		want  string
	}{
		{
			name:  "simple name with no alias",
			addon: Addon{Name: "Simple", Version: "1.0"},
			want:  "{\n    [\"Simple\"] = {name=\"Simple\", version=\"1.0\"}, \n}\n",
		},
		{
			name: "quotes slashes and line breaks",
			addon: Addon{
				Name:    "O'Brien \"Beta\"\\Pack\nLine",
				Alias:   "Alias \"quoted\"\\dir\r\nNext",
				Version: "v1\n2\x00",
			},
			want: "{\n    [\"O'Brien \\\"Beta\\\"\\\\Pack\\010Line\"] = {name=\"Alias \\\"quoted\\\"\\\\dir\\013\\010Next\", version=\"v1\\0102\\000\"}, \n}\n",
		},
		{
			name:  "unambiguous decimal escape before digits",
			addon: Addon{Name: "Line\n123", Version: "\t45"},
			want:  "{\n    [\"Line\\010123\"] = {name=\"Line\\010123\", version=\"\\00945\"}, \n}\n",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := string(generateUpdatesLua(map[string]Addon{tt.addon.Name: tt.addon}))
			if got != tt.want {
				t.Errorf("generated Lua = %q, want %q", got, tt.want)
			}
		})
	}
}

func assertNoUpdateTempFiles(t *testing.T, dir string) {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("ReadDir(%q): %v", dir, err)
	}
	for _, e := range entries {
		if strings.Contains(e.Name(), ".tmp-") {
			t.Fatalf("leftover temp file %q in %s", e.Name(), dir)
		}
	}
}

func TestGenerateUpdateAddonLuaWritesAtomically(t *testing.T) {
	addonsTxtPath := setupAddonsTxtTest(t)
	if err := file.WriteLines(addonsTxtPath, []string{"SomeAddon"}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}

	addonDir, err := config.GetAddonDir()
	if err != nil {
		t.Fatalf("GetAddonDir: %v", err)
	}
	addonPath := filepath.Join(addonDir, updateNotification)
	if err := os.MkdirAll(addonPath, 0755); err != nil {
		t.Fatalf("MkdirAll: %v", err)
	}
	if err := os.WriteFile(filepath.Join(addonPath, "main.lua"), []byte("old main"), 0644); err != nil {
		t.Fatalf("seed main.lua: %v", err)
	}
	if err := os.WriteFile(filepath.Join(addonPath, "updates.lua"), []byte("old updates"), 0644); err != nil {
		t.Fatalf("seed updates.lua: %v", err)
	}

	updates := map[string]Addon{"SomeAddon": {Name: "SomeAddon", Version: "2.0"}}
	GenerateUpdateAddonLua(updates)

	gotMain, err := os.ReadFile(filepath.Join(addonPath, "main.lua"))
	if err != nil {
		t.Fatalf("read main.lua: %v", err)
	}
	if string(gotMain) != string(luaScript) {
		t.Fatalf("main.lua = %q, want embedded luaScript", gotMain)
	}

	gotUpdates, err := os.ReadFile(filepath.Join(addonPath, "updates.lua"))
	if err != nil {
		t.Fatalf("read updates.lua: %v", err)
	}
	if want := string(generateUpdatesLua(updates)); string(gotUpdates) != want {
		t.Fatalf("updates.lua = %q, want %q", gotUpdates, want)
	}

	assertNoUpdateTempFiles(t, addonPath)
}

func TestGenerateUpdateAddonLuaEmptyUpdatesRemovesUpdatesLua(t *testing.T) {
	addonsTxtPath := setupAddonsTxtTest(t)
	if err := file.WriteLines(addonsTxtPath, []string{"SomeAddon"}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}

	addonDir, err := config.GetAddonDir()
	if err != nil {
		t.Fatalf("GetAddonDir: %v", err)
	}
	addonPath := filepath.Join(addonDir, updateNotification)
	if err := os.MkdirAll(addonPath, 0755); err != nil {
		t.Fatalf("MkdirAll: %v", err)
	}
	if err := os.WriteFile(filepath.Join(addonPath, "updates.lua"), []byte("old updates"), 0644); err != nil {
		t.Fatalf("seed updates.lua: %v", err)
	}

	GenerateUpdateAddonLua(map[string]Addon{})

	if _, err := os.Stat(filepath.Join(addonPath, "updates.lua")); !os.IsNotExist(err) {
		t.Fatalf("updates.lua should be removed, stat err = %v", err)
	}

	gotMain, err := os.ReadFile(filepath.Join(addonPath, "main.lua"))
	if err != nil {
		t.Fatalf("read main.lua: %v", err)
	}
	if string(gotMain) != string(luaScript) {
		t.Fatalf("main.lua = %q, want embedded luaScript", gotMain)
	}

	assertNoUpdateTempFiles(t, addonPath)
}

func TestGenerateUpdateAddonLuaFailedWritePreservesUpdatesLua(t *testing.T) {
	addonsTxtPath := setupAddonsTxtTest(t)
	if err := file.WriteLines(addonsTxtPath, []string{"SomeAddon"}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}

	addonDir, err := config.GetAddonDir()
	if err != nil {
		t.Fatalf("GetAddonDir: %v", err)
	}
	addonPath := filepath.Join(addonDir, updateNotification)
	if err := os.MkdirAll(addonPath, 0755); err != nil {
		t.Fatalf("MkdirAll: %v", err)
	}
	// A directory named main.lua makes the atomic rename fail portably.
	if err := os.Mkdir(filepath.Join(addonPath, "main.lua"), 0755); err != nil {
		t.Fatalf("mkdir main.lua: %v", err)
	}
	if err := os.WriteFile(filepath.Join(addonPath, "updates.lua"), []byte("old updates"), 0644); err != nil {
		t.Fatalf("seed updates.lua: %v", err)
	}

	GenerateUpdateAddonLua(map[string]Addon{"SomeAddon": {Name: "SomeAddon", Version: "2.0"}})

	if info, err := os.Stat(filepath.Join(addonPath, "main.lua")); err != nil || !info.IsDir() {
		t.Fatalf("main.lua should still be a directory, info = %v, err = %v", info, err)
	}

	gotUpdates, err := os.ReadFile(filepath.Join(addonPath, "updates.lua"))
	if err != nil {
		t.Fatalf("read updates.lua: %v", err)
	}
	if string(gotUpdates) != "old updates" {
		t.Fatalf("updates.lua = %q, want %q", gotUpdates, "old updates")
	}

	assertNoUpdateTempFiles(t, addonPath)
}
