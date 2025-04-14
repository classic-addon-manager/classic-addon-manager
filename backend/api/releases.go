package api

import (
	"ClassicAddonManager/backend/logger"
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"strconv"
	"time"

	"github.com/mitchellh/mapstructure"
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

	data := apiResponse.Data.(map[string]any)
	r := data["release"].(map[string]any)
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
	if err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return Release{}, err
	}

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

	data := apiResponse.Data.(map[string]any)
	release := ApplicationRelease{
		Version: data["version"].(string),
		Url:     data["url"].(string),
	}

	return release, nil
}

// GetLatestReleasesBulk fetches the latest release information for multiple addons via a single POST request.
func GetLatestReleasesBulk(names []string) (map[string]Release, error) {
	releasesMap := make(map[string]Release)

	// Prepare the request body
	requestBody, err := json.Marshal(map[string][]string{
		"addons": names,
	})
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error marshaling request body:", err)
		return nil, err // Return error if JSON marshaling fails
	}

	// Create the POST request
	url := ApiURL + "/latest_releases"
	req, err := http.NewRequest("POST", url, bytes.NewBuffer(requestBody))
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error creating request:", err)
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Client", GetApiClientHeader())

	// Send the request
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error sending request:", err)
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		errMsg := "GetLatestReleasesBulk Error: Status Code " + strconv.Itoa(resp.StatusCode)
		bodyBytes, readErr := io.ReadAll(resp.Body)
		if readErr == nil && len(bodyBytes) > 0 {
			errMsg += " Body: " + string(bodyBytes)
		}
		logger.Error(errMsg, errors.New("unexpected status code: "+strconv.Itoa(resp.StatusCode)))
		return nil, errors.New("failed to fetch bulk releases, status: " + strconv.Itoa(resp.StatusCode))
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetLatestReleasesBulk Error reading response body:", err)
		return nil, err
	}

	// Unmarshal the API response
	var apiResponse ApiResponse
	if err := json.Unmarshal(body, &apiResponse); err != nil {
		logger.Error("GetLatestReleasesBulk Error unmarshaling response:", err)
		return nil, err
	}

	// Check API response status
	if !apiResponse.Status {
		logger.Warn("GetLatestReleasesBulk API status false: " + apiResponse.Message)
		return nil, errors.New(apiResponse.Message)
	}

	// Process the data field, expecting map[string]any where keys are addon names
	responseData, ok := apiResponse.Data.(map[string]any)
	if !ok {
		errMsg := "GetLatestReleasesBulk Error: could not cast response data to map[string]any"
		logger.Error(errMsg, errors.New("type assertion failed"))
		return nil, errors.New(errMsg)
	}

	// Iterate through the response data and decode each addon's release info
	for name, releaseDataAny := range responseData {
		releaseDataMap, ok := releaseDataAny.(map[string]any)
		if !ok {
			logger.Warn("Skipping addon " + name + ": could not cast release data to map[string]any")
			continue // Skip this entry if data format is unexpected
		}

		// Check for nested "release" and "tag" structure like in GetLatestAddonRelease
		releaseSubMapAny, releaseOk := releaseDataMap["release"]
		tagSubMapAny, tagOk := releaseDataMap["tag"]

		if !releaseOk || !tagOk {
			logger.Warn("Skipping addon " + name + ": missing 'release' or 'tag' key in response data")
			continue
		}

		releaseSubMap, releaseMapOk := releaseSubMapAny.(map[string]any)
		tagSubMap, tagMapOk := tagSubMapAny.(map[string]any)

		if !releaseMapOk || !tagMapOk {
			logger.Warn("Skipping addon " + name + ": could not cast 'release' or 'tag' data to map[string]any")
			continue
		}

		var release Release
		// Decode the release sub-map using mapstructure
		decoderConfigRelease := &mapstructure.DecoderConfig{
			Result: &release,
			// Add hook to parse time string correctly from the "release" sub-map
			DecodeHook: mapstructure.ComposeDecodeHookFunc(
				mapstructure.StringToTimeHookFunc(time.RFC3339),
			),
			TagName: "json", // Assuming the API response uses JSON tags if Release struct has them
		}
		decoderRelease, err := mapstructure.NewDecoder(decoderConfigRelease)
		if err != nil {
			logger.Error("GetLatestReleasesBulk Error creating mapstructure decoder for release "+name+":", err)
			continue // Skip on decoder error
		}

		if err := decoderRelease.Decode(releaseSubMap); err != nil {
			logger.Error("GetLatestReleasesBulk Error decoding release sub-map for "+name+":", err)
			continue // Skip this addon if decoding fails
		}

		// Decode the tag sub-map
		var tag Tag
		decoderConfigTag := &mapstructure.DecoderConfig{
			Result:  &tag,
			TagName: "json", // Assuming Tag struct also uses json tags
		}
		decoderTag, err := mapstructure.NewDecoder(decoderConfigTag)
		if err != nil {
			logger.Error("GetLatestReleasesBulk Error creating mapstructure decoder for tag "+name+":", err)
			continue
		}
		if err := decoderTag.Decode(tagSubMap); err != nil {
			logger.Error("GetLatestReleasesBulk Error decoding tag sub-map for "+name+":", err)
			continue
		}

		release.Tag = tag // Assign the decoded tag to the release

		releasesMap[name] = release
	}

	return releasesMap, nil
}
