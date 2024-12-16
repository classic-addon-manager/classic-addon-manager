package util

import (
	"ClassicAddonManager/backend/logger"
	"io"
	"net/http"
	"os"
)

func DownloadFile(url string, path string) error {
	resp, err := http.Get(url)
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
