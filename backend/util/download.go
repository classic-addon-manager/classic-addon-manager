package util

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/logger"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"time"
)

var downloadClient = &http.Client{Timeout: 10 * time.Minute}

func DownloadFile(url string, path string) error {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return err
	}
	req.Header.Set("X-Client", api.GetClientHeader())
	req.Header.Set("Accept", "application/octet-stream, */*")
	if token := auth.GetToken(); token != "" {
		req.Header.Set("X-Token", token)
	}

	resp, err := downloadClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("download failed with status code %d", resp.StatusCode)
	}

	tmp, err := os.CreateTemp(filepath.Dir(path), filepath.Base(path)+".*.part")
	if err != nil {
		return err
	}
	tmpName := tmp.Name()

	if _, err := io.Copy(tmp, resp.Body); err != nil {
		tmp.Close()
		os.Remove(tmpName)
		return fmt.Errorf("downloading %s: %w", url, err)
	}
	if err := tmp.Close(); err != nil {
		os.Remove(tmpName)
		return fmt.Errorf("closing %s: %w", tmpName, err)
	}
	if err := os.Rename(tmpName, path); err != nil {
		os.Remove(tmpName)
		return fmt.Errorf("replacing %s: %w", path, err)
	}

	logger.Info("Downloaded " + path)
	return nil
}
