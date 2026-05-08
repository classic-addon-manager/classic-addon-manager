package auth

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/logger"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

type sessionFile struct {
	Version   int       `json:"version"`
	Encoding  string    `json:"encoding"`
	Token     string    `json:"token"`
	UpdatedAt time.Time `json:"updated_at"`
}

const sessionFileName = "auth_session.json"

func sessionPath() string {
	return filepath.Join(config.GetDataDir(), sessionFileName)
}

func tmpSessionPath() string {
	return filepath.Join(config.GetDataDir(), sessionFileName+".tmp")
}

func LoadFromDisk() {
	path := sessionPath()
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			logger.Info("Auth: no session file, starting signed out")
			return
		}
		logger.Error("Auth: error reading session file:", err)
		quarantine(path)
		return
	}

	var sf sessionFile
	if err := json.Unmarshal(data, &sf); err != nil {
		logger.Error("Auth: error parsing session file:", err)
		quarantine(path)
		return
	}

	if sf.Version != 1 || sf.Token == "" {
		logger.Warn("Auth: invalid session file (bad version or empty token)")
		quarantine(path)
		return
	}

	plainToken, err := unprotectToken(sf.Encoding, sf.Token)
	if err != nil {
		logger.Error("Auth: error decrypting token:", err)
		quarantine(path)
		return
	}

	SetToken(plainToken)
	logger.Info("Auth: session loaded from disk")
}

func SaveToDisk(token string) error {
	if token == "" {
		return fmt.Errorf("token must not be empty")
	}

	protected, encoding, err := protectToken(token)
	if err != nil {
		return fmt.Errorf("error protecting token: %w", err)
	}

	sf := sessionFile{
		Version:   1,
		Encoding:  encoding,
		Token:     protected,
		UpdatedAt: time.Now().UTC(),
	}

	data, err := json.MarshalIndent(sf, "", "  ")
	if err != nil {
		return fmt.Errorf("error marshaling session file: %w", err)
	}

	tmpPath := tmpSessionPath()
	finalPath := sessionPath()
	if err := writeAtomic(tmpPath, finalPath, data); err != nil {
		return fmt.Errorf("error writing session file: %w", err)
	}

	SetToken(token)
	logger.Info("Auth: session saved to disk")
	return nil
}

func DeleteFromDisk() error {
	path := sessionPath()
	err := os.Remove(path)
	if err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("error deleting session file: %w", err)
	}
	ClearToken()
	logger.Info("Auth: session deleted from disk")
	return nil
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

	return afterWriteHook(finalPath)
}

func quarantine(path string) {
	ts := time.Now().Format("20060102150405")
	newPath := filepath.Join(config.GetDataDir(), fmt.Sprintf("auth_session.invalid.%s.json", ts))
	if err := os.Rename(path, newPath); err != nil {
		logger.Error("Auth: error quarantining invalid session file:", err)
	} else {
		logger.Info(fmt.Sprintf("Auth: quarantined invalid session file to %s", newPath))
	}
	ClearToken()
}
