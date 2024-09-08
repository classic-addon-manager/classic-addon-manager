package api

import (
	"ClassicAddonManager/logger"
	"encoding/json"
	"errors"
	"github.com/mitchellh/mapstructure"
	"io"
	"net/http"
	"strconv"
	"time"
)

func GetLatestRelease(name string) (Release, error) {
	url := apiUrl + "/latest_release/" + name

	resp, err := http.Get(url)
	if err != nil {
		return Release{}, err
	}
	defer resp.Body.Close()

	// If the status code is not 200 here, there's likely no release
	if resp.StatusCode != 200 {
		logger.Error("GetLatestRelease Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return Release{}, errors.New("no release found")
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetLatestRelease Error:", err)
		return Release{}, err
	}

	var apiResponse ApiResponse

	if err := json.Unmarshal(body, &apiResponse); err != nil {
		logger.Error("GetLatestRelease Error:", err)
		return Release{}, err
	}

	if !apiResponse.Status {
		logger.Warn("GetLatestRelease status false: " + apiResponse.Message)
		return Release{}, errors.New(apiResponse.Message)
	}

	data := apiResponse.Data.(map[string]interface{})
	r := data["release"].(map[string]interface{})
	release := Release{
		ZipballUrl:  r["zipball_url"].(string),
		TagName:     r["tag_name"].(string),
		Body:        r["body"].(string),
		PublishedAt: time.Time{},
		Tag:         Tag{},
	}

	// Parse time from data.release.published_at
	publishedAtStr := r["published_at"].(string)
	release.PublishedAt, err = time.Parse(time.RFC3339, publishedAtStr)

	tag := Tag{}
	err = mapstructure.Decode(data["tag"], &tag)
	if err != nil {
		logger.Error("GetLatestRelease Error:", err)
		return Release{}, err
	}

	release.Tag = tag
	return release, nil
}
