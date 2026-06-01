package appstate

import (
	"ClassicAddonManager/backend/config"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func withTestStateDir(t *testing.T) func() {
	t.Helper()
	dir := t.TempDir()
	prev := getStateDir
	getStateDir = func() string { return dir }
	return func() {
		getStateDir = prev
	}
}

func writeStateFile(t *testing.T, sf stateFile) {
	t.Helper()
	data, err := json.MarshalIndent(sf, "", "  ")
	if err != nil {
		t.Fatalf("marshal state: %v", err)
	}
	path := filepath.Join(getStateDir(), stateFileName)
	if err := os.WriteFile(path, data, 0600); err != nil {
		t.Fatalf("write state file: %v", err)
	}
}

func TestShouldShowKofiModal_NoFile(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	if ShouldShowKofiModal() {
		t.Fatal("expected no show on first launch when state file missing")
	}
}

func TestEnsureInitialized_CreatesFile(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	if err := EnsureInitialized(); err != nil {
		t.Fatalf("ensure state: %v", err)
	}
	if !stateFileExists() {
		t.Fatal("expected state file after ensure")
	}
	if err := EnsureInitialized(); err != nil {
		t.Fatalf("ensure state again: %v", err)
	}
}

func TestFirstLaunchThenSecondLaunch_ShowsModal(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	if ShouldShowKofiModal() {
		t.Fatal("first launch should not show kofi modal")
	}
	if err := EnsureInitialized(); err != nil {
		t.Fatalf("ensure state: %v", err)
	}
	if !ShouldShowKofiModal() {
		t.Fatal("second launch should show kofi modal before last shown recorded")
	}
}

func TestShouldShowKofiModal_NeverShown(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	writeStateFile(t, stateFile{Version: 1})

	if !ShouldShowKofiModal() {
		t.Fatal("expected show when kofi_modal_last_shown_at unset")
	}
}

func TestShouldShowKofiModal_WithinThreeMonths(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	shown := time.Now().UTC().AddDate(0, -2, 0)
	writeStateFile(t, stateFile{Version: 1, KofiModalLastShownAt: &shown})

	if ShouldShowKofiModal() {
		t.Fatal("expected no show within three months")
	}
}

func TestShouldShowKofiModal_AfterThreeMonths(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	shown := time.Now().UTC().AddDate(0, -4, 0)
	writeStateFile(t, stateFile{Version: 1, KofiModalLastShownAt: &shown})

	if !ShouldShowKofiModal() {
		t.Fatal("expected show after three months")
	}
}

func TestShouldShowKofiModal_CorruptFile(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	path := filepath.Join(getStateDir(), stateFileName)
	if err := os.WriteFile(path, []byte("{not json"), 0600); err != nil {
		t.Fatalf("write corrupt state: %v", err)
	}

	if !ShouldShowKofiModal() {
		t.Fatal("expected show on corrupt state (fail open)")
	}
}

func TestShouldShowKofiModal_UnsupportedVersion(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	writeStateFile(t, stateFile{Version: 2})

	if !ShouldShowKofiModal() {
		t.Fatal("expected show on unsupported version (fail open)")
	}
}

func TestRecordKofiModalShown_WritesState(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	if err := RecordKofiModalShown(); err != nil {
		t.Fatalf("record shown: %v", err)
	}

	if ShouldShowKofiModal() {
		t.Fatal("expected no show immediately after recording")
	}

	data, err := os.ReadFile(statePath())
	if err != nil {
		t.Fatalf("read state file: %v", err)
	}

	var sf stateFile
	if err := json.Unmarshal(data, &sf); err != nil {
		t.Fatalf("parse state file: %v", err)
	}
	if sf.Version != 1 {
		t.Fatalf("version = %d, want 1", sf.Version)
	}
	if sf.KofiModalLastShownAt == nil {
		t.Fatal("kofi_modal_last_shown_at not set")
	}
	if time.Since(*sf.KofiModalLastShownAt) > time.Minute {
		t.Fatal("kofi_modal_last_shown_at too old")
	}
}

func TestRecordKofiModalShown_UpdatesExisting(t *testing.T) {
	cleanup := withTestStateDir(t)
	defer cleanup()

	old := time.Now().UTC().AddDate(-1, 0, 0)
	writeStateFile(t, stateFile{Version: 1, KofiModalLastShownAt: &old})

	if err := RecordKofiModalShown(); err != nil {
		t.Fatalf("record shown: %v", err)
	}

	sf, err := loadState()
	if err != nil {
		t.Fatalf("load state: %v", err)
	}
	if sf.KofiModalLastShownAt == nil || !sf.KofiModalLastShownAt.After(old) {
		t.Fatal("expected kofi_modal_last_shown_at to be updated")
	}
}

func TestStatePath_UsesConfigDataDirByDefault(t *testing.T) {
	expected := filepath.Join(config.GetDataDir(), stateFileName)
	if statePath() != expected {
		t.Fatalf("statePath() = %q, want %q", statePath(), expected)
	}
}
