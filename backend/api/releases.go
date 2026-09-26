package api

import (
	"ClassicAddonManager/backend/logger"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"time"
)

func GetAddonRelease(name string, version string) (Release, error) {
	path := fmt.Sprintf("/addon/%s/release/%s", name, version)

	req, err := NewApiRequest(nil, http.MethodGet, path, nil)
	if err != nil {
		return Release{}, err
	}
	resp, err := Client.Do(req)
	if err != nil {
		return Release{}, err
	}
	defer resp.Body.Close()

	// If the status code is not 200 here, there's likely no release
	if resp.StatusCode != 200 {
		logger.Error("GetAddonRelease Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return Release{}, errors.New("no release found")
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetAddonRelease Error:", err)
		return Release{}, err
	}

	var apiResponse ApiResponse

	if err := json.Unmarshal(body, &apiResponse); err != nil {
		logger.Error("GetAddonRelease Error:", err)
		return Release{}, err
	}

	if !apiResponse.Status {
		logger.Warn("GetAddonRelease status false: " + apiResponse.Message)
		return Release{}, errors.New(apiResponse.Message)
	}

	var data struct {
		Release *struct {
			ZipballUrl  string    `json:"zipball_url"`
			TagName     string    `json:"tag_name"`
			Body        string    `json:"body"`
			PublishedAt time.Time `json:"published_at"`
		} `json:"release"`
		Tag Tag `json:"tag"`
	}
	if err := json.Unmarshal(apiResponse.Data, &data); err != nil {
		logger.Error("GetAddonRelease Error:", err)
		return Release{}, err
	}

	if data.Release == nil || data.Release.ZipballUrl == "" || data.Release.TagName == "" || data.Release.PublishedAt.IsZero() {
		err := errors.New("release response is missing required fields")
		logger.Error("GetAddonRelease Error:", err)
		return Release{}, err
	}

	return Release{
		ZipballUrl:  data.Release.ZipballUrl,
		TagName:     data.Release.TagName,
		Body:        data.Release.Body,
		PublishedAt: data.Release.PublishedAt,
		Tag:         data.Tag,
	}, nil
}

func GetLatestApplicationRelease() (ApplicationRelease, error) {
	req, err := NewApiRequest(nil, http.MethodGet, "/latest_application_release", nil)
	if err != nil {
		return ApplicationRelease{}, err
	}
	resp, err := Client.Do(req)
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
		logger.Error("GetLatestApplicationRelease Error:", err)
		return ApplicationRelease{}, err
	}

	var apiResponse ApiResponse

	if err := json.Unmarshal(body, &apiResponse); err != nil {
		logger.Error("GetLatestApplicationRelease Error:", err)
		return ApplicationRelease{}, err
	}

	if !apiResponse.Status {
		logger.Warn("GetLatestApplicationRelease status false: " + apiResponse.Message)
		return ApplicationRelease{}, errors.New(apiResponse.Message)
	}

	var release ApplicationRelease
	if err := json.Unmarshal(apiResponse.Data, &release); err != nil {
		logger.Error("GetLatestApplicationRelease Error:", err)
		return ApplicationRelease{}, err
	}

	if release.Version == "" || release.Url == "" {
		err := errors.New("application release response is missing required fields")
		logger.Error("GetLatestApplicationRelease Error:", err)
		return ApplicationRelease{}, err
	}

	return release, nil
}

// GetLatestReleasesBulk fetches the latest release information for multiple addons via a single POST request.
func GetLatestReleasesBulk(names []string) (map[string]Release, error) {
	// Prepare the request body
	requestBody, err := json.Marshal(map[string][]string{
		"addons": names,
	})
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error marshaling request body:", err)
		return nil, err
	}

	// Create and send the POST request
	req, err := NewApiRequest(nil, http.MethodPost, "/latest_releases", bytes.NewBuffer(requestBody))
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error creating request:", err)
		return nil, err
	}

	resp, err := Client.Do(req)
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error sending request:", err)
		return nil, err
	}
	defer resp.Body.Close()

	// Handle non-200 responses
	if resp.StatusCode != 200 {
		bodyBytes, _ := io.ReadAll(resp.Body)
		errMsg := "GetLatestReleasesBulk Error: Status Code " + strconv.Itoa(resp.StatusCode)
		if len(bodyBytes) > 0 {
			errMsg += " Body: " + string(bodyBytes)
		}
		logger.Error(errMsg, errors.New("unexpected status code"))
		return nil, errors.New("failed to fetch bulk releases, status: " + strconv.Itoa(resp.StatusCode))
	}

	// Parse the API response
	var apiResponse struct {
		Status  bool   `json:"status"`
		Message string `json:"message"`
		Data    map[string]struct {
			Release struct {
				ZipballUrl  string    `json:"zipball_url"`
				TagName     string    `json:"tag_name"`
				Body        string    `json:"body"`
				PublishedAt time.Time `json:"published_at"`
			} `json:"release"`
			Tag Tag `json:"tag"`
		} `json:"data"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&apiResponse); err != nil {
		logger.Error("GetLatestReleasesBulk Error unmarshaling response:", err)
		return nil, err
	}

	if !apiResponse.Status {
		logger.Warn("GetLatestReleasesBulk API status false: " + apiResponse.Message)
		return nil, errors.New(apiResponse.Message)
	}

	// Convert the response data into the expected format
	releases := make(map[string]Release)
	for name, data := range apiResponse.Data {
		releases[name] = Release{
			ZipballUrl:  data.Release.ZipballUrl,
			TagName:     data.Release.TagName,
			Body:        data.Release.Body,
			PublishedAt: data.Release.PublishedAt,
			Tag:         data.Tag,
		}
	}

	return releases, nil
}
