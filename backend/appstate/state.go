package appstate

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/logger"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

const stateFileName = "app_state.json"

// getStateDir resolves the directory for app_state.json. Tests override via setStateDir.
var getStateDir = config.GetDataDir

type stateFile struct {
	Version              int        `json:"version"`
	KofiModalLastShownAt *time.Time `json:"kofi_modal_last_shown_at,omitempty"`
}

func statePath() string {
	return filepath.Join(getStateDir(), stateFileName)
}

func tmpStatePath() string {
	return filepath.Join(getStateDir(), stateFileName+".tmp")
}

func stateFileExists() bool {
	_, err := os.Stat(statePath())
	return err == nil
}

func ShouldShowKofiModal() bool {
	if !stateFileExists() {
		return false
	}
	sf, err := loadState()
	if err != nil {
		logger.Warn(fmt.Sprintf("App state: could not load state, showing kofi modal: %v", err))
		return true
	}
	if sf.KofiModalLastShownAt == nil {
		return true
	}
	return time.Now().UTC().After(sf.KofiModalLastShownAt.AddDate(0, 3, 0))
}

// EnsureInitialized creates app_state.json on first launch without showing the kofi modal.
func EnsureInitialized() error {
	if stateFileExists() {
		return nil
	}
	sf := stateFile{Version: 1}
	data, err := json.MarshalIndent(sf, "", "  ")
	if err != nil {
		return fmt.Errorf("error marshaling app state: %w", err)
	}
	if err := writeAtomic(tmpStatePath(), statePath(), data); err != nil {
		return fmt.Errorf("error writing app state: %w", err)
	}
	logger.Info("App state: initialized app_state.json (first launch)")
	return nil
}

func RecordKofiModalShown() error {
	now := time.Now().UTC()
	sf := stateFile{
		Version:              1,
		KofiModalLastShownAt: &now,
	}

	data, err := json.MarshalIndent(sf, "", "  ")
	if err != nil {
		return fmt.Errorf("error marshaling app state: %w", err)
	}

	if err := writeAtomic(tmpStatePath(), statePath(), data); err != nil {
		return fmt.Errorf("error writing app state: %w", err)
	}

	logger.Info("App state: kofi modal last shown recorded")
	return nil
}

func loadState() (*stateFile, error) {
	path := statePath()
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return &stateFile{}, nil
		}
		return nil, err
	}

	var sf stateFile
	if err := json.Unmarshal(data, &sf); err != nil {
		return nil, err
	}
	if sf.Version != 1 {
		return nil, fmt.Errorf("unsupported app state version: %d", sf.Version)
	}
	return &sf, nil
}

func writeAtomic(tmpPath, finalPath string, data []byte) error {
	if err := os.WriteFile(tmpPath, data, 0600); err != nil {
		return fmt.Errorf("error writing temp file: %w", err)
	}

	f, err := os.Open(tmpPath)
	if err == nil {
		_ = f.Sync()
		f.Close()
	}

	if err := os.Rename(tmpPath, finalPath); err != nil {
		return fmt.Errorf("error renaming temp file: %w", err)
	}

	return nil
}
