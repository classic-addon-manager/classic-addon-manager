package util

import (
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
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
	if shared.AuthToken != "" {
		req.Header.Set("X-Token", shared.AuthToken)
	}
	req.Header.Set("X-Client", "Classic Addon Manager v"+shared.Version)
	client := &http.Client{}
	// Send the request
	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	out, err := os.Create(path)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	logger.Info("Downloaded" + path)
	return err
}
