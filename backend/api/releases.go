package api

import (
	"ClassicAddonManager/backend/logger"
	"encoding/json"
	"errors"
	"github.com/mitchellh/mapstructure"
	"io"
	"net/http"
	"strconv"
	"time"
)

func GetLatestAddonRelease(name string) (Release, error) {
	url := ApiURL + "/latest_release/" + name

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return Release{}, err
	}
	req.Header.Set("X-Client", GetApiClientHeader())
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return Release{}, err
	}
	defer resp.Body.Close()

	// If the status code is not 200 here, there's likely no release
	if resp.StatusCode != 200 {
		logger.Error("GetLatestAddonRelease Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return Release{}, errors.New("no release found")
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return Release{}, err
	}

	var apiResponse ApiResponse

	if err := json.Unmarshal(body, &apiResponse); err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return Release{}, err
	}

	if !apiResponse.Status {
		logger.Warn("GetLatestAddonRelease status false: " + apiResponse.Message)
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
		logger.Error("GetLatestAddonRelease Error:", err)
		return Release{}, err
	}

	release.Tag = tag
	return release, nil
}

func GetLatestApplicationRelease() (ApplicationRelease, error) {
	url := ApiURL + "/latest_application_release"

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return ApplicationRelease{}, err
	}
	req.Header.Set("X-Client", GetApiClientHeader())
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return ApplicationRelease{}, err
	}
	defer resp.Body.Close()

	// If the status code is not 200 here, there's likely no release
	if resp.StatusCode != 200 {
		logger.Error("GetLatestApplicationRelease Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return ApplicationRelease{}, errors.New("no release found")
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return ApplicationRelease{}, err
	}

	var apiResponse ApiResponse

	if err := json.Unmarshal(body, &apiResponse); err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return ApplicationRelease{}, err
	}

	if !apiResponse.Status {
		logger.Warn("GetLatestAddonRelease status false: " + apiResponse.Message)
		return ApplicationRelease{}, errors.New(apiResponse.Message)
	}

	data := apiResponse.Data.(map[string]interface{})
	release := ApplicationRelease{
		Version: data["version"].(string),
		Url:     data["url"].(string),
	}

	return release, nil
}
