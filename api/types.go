package api

import "time"

type ApiResponse struct {
	Status  bool        `json:"status"`
	Message string      `json:"message"`
	Data    interface{} `json:"data"`
}

type Release struct {
	ZipballUrl  string    `json:"zipball_url"`
	TagName     string    `json:"tag_name"`
	Body        string    `json:"body"`
	PublishedAt time.Time `json:"published_at"`
	Tag         Tag       `json:"tag"`
}

type Tag struct {
	Ref  string `json:"ref"`
	Sha  string `json:"sha"`
	Type string `json:"type"`
	Url  string `json:"url"`
}

type ApplicationRelease struct {
	Version string `json:"version"`
	Url     string `json:"url"`
}
