package util

import (
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"fmt"
	"io"
	"net/http"
	"os"
)

func DownloadFile(url string, path string) error {
	// Make a get request containing a token if there is one.
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return err
	}
	// Set the authorization header if the token is set
	if token := auth.GetToken(); token != "" {
		req.Header.Set("X-Token", token)
	}
	req.Header.Set("X-Client", "Classic Addon Manager v"+shared.Version)
	client := &http.Client{}
	// Send the request
	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 300 {
		return fmt.Errorf("download failed with status code %d", resp.StatusCode)
	}

	out, err := os.Create(path)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	logger.Info("Downloaded" + path)
	return err
}
