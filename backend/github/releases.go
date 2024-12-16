package github

import (
	"ClassicAddonManager/backend/logger"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"strconv"
	"time"
)

var githubApi = "https://api.github.com"

type GithubRelease struct {
	ZipballURL  string    `json:"zipball_url"`
	TagName     string    `json:"tag_name"`
	Body        string    `json:"body"`
	PublishedAt time.Time `json:"published_at"`
	Tag         GithubTag `json:"tag"`
}

type GithubTag struct {
	Ref  string `json:"ref"`
	Sha  string `json:"sha"`
	Type string `json:"type"`
	Url  string `json:"url"`
}

// GetLatestAddonRelease returns the latest zipball release of a GitHub repository
func GetLatestAddonRelease(repo string) (GithubRelease, error) {
	// Get the latest release from the GitHub API
	//https://api.github.com/repos/OWNER/REPO/releases
	url := githubApi + "/repos/" + repo + "/releases/latest"

	resp, err := http.Get(url)
	if err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return GithubRelease{}, err
	}
	defer resp.Body.Close()

	// If the status code is not 200 here, there's likely no release
	if resp.StatusCode != 200 {
		logger.Error("GetLatestAddonRelease Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return GithubRelease{}, errors.New("no release found")
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return GithubRelease{}, err
	}

	var release GithubRelease

	if err := json.Unmarshal(body, &release); err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return GithubRelease{}, err
	}

	tag, err := getTagByRelease(repo, &release)
	if err != nil {
		logger.Error("GetLatestAddonRelease Error:", err)
		return GithubRelease{}, err
	}

	release.Tag = tag
	return release, nil
}

func GetReleases(repo string) ([]GithubRelease, error) {
	url := githubApi + "/repos/" + repo + "/releases"

	resp, err := http.Get(url)
	if err != nil {
		logger.Error("GetReleases Error:", err)
		return []GithubRelease{}, err
	}
	defer resp.Body.Close()

	// If the status code is not 200 here, there's likely no release
	if resp.StatusCode != 200 {
		logger.Error("GetReleases Error: Status Code", errors.New(strconv.Itoa(resp.StatusCode)))
		return []GithubRelease{}, errors.New("no release found")
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		logger.Error("GetReleases Error:", err)
		return []GithubRelease{}, err
	}

	var releases []GithubRelease

	if err := json.Unmarshal(body, &releases); err != nil {
		logger.Error("GetReleaes Error:", err)
		return []GithubRelease{}, err
	}

	for i := range releases {
		release := &releases[i]
		tag, err := getTagByRelease(repo, release)
		if err != nil {
			logger.Error("GetReleases Error:", err)
			return []GithubRelease{}, err
		}
		release.Tag = tag
	}

	return releases, nil
}

func getTagByRelease(repo string, release *GithubRelease) (GithubTag, error) {
	url := githubApi + "/repos/" + repo + "/git/refs/tags/" + release.TagName
	r, err := http.Get(url)
	if err != nil {
		logger.Error("getTagByRelease Error:", err)
		return GithubTag{}, err
	}
	defer r.Body.Close()

	if r.StatusCode != 200 {
		logger.Error("getTagByRelease Error: Status Code", errors.New(strconv.Itoa(r.StatusCode)))
		return GithubTag{}, errors.New("no release found")
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		logger.Error("getTagByRelease Error:", err)
		return GithubTag{}, err
	}

	var tag GithubTag

	if err := json.Unmarshal(body, &tag); err != nil {
		logger.Error("getTagByRelease Error:", err)
		return GithubTag{}, err
	}

	return tag, nil
}
