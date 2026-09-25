package addon

import (
	"encoding/json"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
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

type roundTripFunc func(*http.Request) (*http.Response, error)

func (f roundTripFunc) RoundTrip(req *http.Request) (*http.Response, error) {
	return f(req)
}

func stubAPIClient(t *testing.T, transport http.RoundTripper) {
	t.Helper()
	original := api.Client
	api.Client = &http.Client{Transport: transport}
	t.Cleanup(func() { api.Client = original })
}

// setupCheckForUpdatesDataDir points the config dir at a fresh temp dir and
// resets the managed addon cache. The logger singleton opens app.log on first
// use and never closes it, so it is initialized while the config dir is the
// shared temp dir (Windows cannot remove a temp dir holding an open file).
func setupCheckForUpdatesDataDir(t *testing.T) {
	t.Helper()
	t.Setenv("APPDATA", os.TempDir())
	t.Setenv("XDG_CONFIG_HOME", os.TempDir())
	logger.Info("starting update check test")

	dataDir := t.TempDir()
	t.Setenv("APPDATA", dataDir)
	t.Setenv("XDG_CONFIG_HOME", dataDir)

	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = nil
		localAddonsMu.Unlock()
	})
}

func seedManagedAddonsFile(t *testing.T, addons ...Addon) {
	t.Helper()
	data, err := json.Marshal(ManagedAddonsFile{Version: ManagedAddonsFileVersion, Addons: addons})
	if err != nil {
		t.Fatalf("marshal managed addons: %v", err)
	}
	fp, err := managedAddonsFilePath()
	if err != nil {
		t.Fatalf("managedAddonsFilePath: %v", err)
	}
	if err := os.WriteFile(fp, data, 0644); err != nil {
		t.Fatalf("write managed_addons.json: %v", err)
	}
}

func bulkReleasesBody(t *testing.T, tags map[string]string) io.Reader {
	t.Helper()
	var b strings.Builder
	b.WriteString(`{"status":true,"data":{`)
	first := true
	for name, tag := range tags {
		if !first {
			b.WriteByte(',')
		}
		first = false
		nameJSON, _ := json.Marshal(name)
		b.Write(nameJSON)
		b.WriteString(`:{"release":{"tag_name":` + `"` + tag + `"` + `,"zipball_url":"","body":"","published_at":"2026-01-01T00:00:00Z"},"tag":{}}`)
	}
	b.WriteString(`}}`)
	return strings.NewReader(b.String())
}

func TestCheckForUpdatesUsesOneBulkRequest(t *testing.T) {
	setupCheckForUpdatesDataDir(t)
	seedManagedAddonsFile(t,
		Addon{Name: "Alpha", Version: "1.0"},
		Addon{Name: "Beta", Version: "1.0"},
		Addon{Name: "Gamma", Version: "3.0"},
	)

	requestCount := 0
	var gotNames []string
	stubAPIClient(t, roundTripFunc(func(req *http.Request) (*http.Response, error) {
		requestCount++
		if req.Method != http.MethodPost {
			t.Errorf("request method = %s, want POST", req.Method)
		}
		if !strings.HasSuffix(req.URL.Path, "/latest_releases") {
			t.Errorf("request path = %s, want suffix /latest_releases", req.URL.Path)
		}
		var body struct {
			Addons []string `json:"addons"`
		}
		if err := json.NewDecoder(req.Body).Decode(&body); err != nil {
			t.Errorf("decode request body: %v", err)
		}
		gotNames = body.Addons
		return &http.Response{
			StatusCode: http.StatusOK,
			Body: io.NopCloser(bulkReleasesBody(t, map[string]string{
				"Alpha": "1.0",
				"Beta":  "2.0",
				"Gamma": "3.1",
			})),
		}, nil
	}))

	updates, err := CheckForUpdates()
	if err != nil {
		t.Fatalf("CheckForUpdates: %v", err)
	}
	if requestCount != 1 {
		t.Fatalf("request count = %d, want 1", requestCount)
	}
	slices.Sort(gotNames)
	if !slices.Equal(gotNames, []string{"Alpha", "Beta", "Gamma"}) {
		t.Fatalf("requested addons = %v, want [Alpha Beta Gamma]", gotNames)
	}
	if len(updates) != 2 {
		t.Fatalf("updates = %v, want 2 entries", updates)
	}
	if got := updates["Beta"]; got.Name != "Beta" || got.Version != "2.0" {
		t.Fatalf("updates[Beta] = %+v, want {Name:Beta Version:2.0}", got)
	}
	if got := updates["Gamma"]; got.Name != "Gamma" || got.Version != "3.1" {
		t.Fatalf("updates[Gamma] = %+v, want {Name:Gamma Version:3.1}", got)
	}
}

func TestCheckForUpdatesReturnsErrorWhenBulkRequestFails(t *testing.T) {
	setupCheckForUpdatesDataDir(t)
	seedManagedAddonsFile(t, Addon{Name: "Alpha", Version: "1.0"})

	stubAPIClient(t, roundTripFunc(func(req *http.Request) (*http.Response, error) {
		return &http.Response{
			StatusCode: http.StatusInternalServerError,
			Body:       io.NopCloser(strings.NewReader("")),
		}, nil
	}))

	updates, err := CheckForUpdates()
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if updates != nil {
		t.Fatalf("updates = %v, want nil on total failure", updates)
	}
}

func TestCheckForUpdatesReportsAddonsMissingFromResponse(t *testing.T) {
	setupCheckForUpdatesDataDir(t)
	seedManagedAddonsFile(t,
		Addon{Name: "Alpha", Version: "1.0"},
		Addon{Name: "Beta", Version: "1.0"},
	)

	stubAPIClient(t, roundTripFunc(func(req *http.Request) (*http.Response, error) {
		return &http.Response{
			StatusCode: http.StatusOK,
			Body:       io.NopCloser(bulkReleasesBody(t, map[string]string{"Alpha": "2.0"})),
		}, nil
	}))

	updates, err := CheckForUpdates()
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !strings.Contains(err.Error(), "Beta") {
		t.Fatalf("error = %q, want it to name Beta", err)
	}
	if len(updates) != 1 {
		t.Fatalf("updates = %v, want 1 entry", updates)
	}
	if got := updates["Alpha"]; got.Name != "Alpha" || got.Version != "2.0" {
		t.Fatalf("updates[Alpha] = %+v, want {Name:Alpha Version:2.0}", got)
	}
}

func TestCheckForUpdatesReturnsErrorWhenManagedAddonsFileMissing(t *testing.T) {
	setupCheckForUpdatesDataDir(t)

	stubAPIClient(t, roundTripFunc(func(req *http.Request) (*http.Response, error) {
		t.Errorf("unexpected request to %s", req.URL)
		return &http.Response{
			StatusCode: http.StatusOK,
			Body:       io.NopCloser(strings.NewReader(`{"status":true,"data":{}}`)),
		}, nil
	}))

	updates, err := CheckForUpdates()
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if updates != nil {
		t.Fatalf("updates = %v, want nil", updates)
	}
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
