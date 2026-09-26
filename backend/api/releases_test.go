package api

import (
	"io"
	"net/http"
	"strings"
	"testing"
	"time"
)

func respondWith(t *testing.T, body string) {
	t.Helper()
	originalClient := Client
	t.Cleanup(func() { Client = originalClient })
	Client = &http.Client{Transport: subscribedAddonsRoundTripper(func(req *http.Request) (*http.Response, error) {
		return &http.Response{
			StatusCode: http.StatusOK,
			Body:       io.NopCloser(strings.NewReader(body)),
		}, nil
	})}
}

func TestGetAddonReleaseDecoding(t *testing.T) {
	validBody := `{"status":true,"data":{"release":{"zipball_url":"https://example.com/a.zip","tag_name":"v1.2.0","body":"notes","published_at":"2024-05-01T12:00:00Z"},"tag":{"ref":"refs/tags/v1.2.0","sha":"abc","type":"commit","url":"https://example.com/tag"}}}`

	t.Run("valid payload", func(t *testing.T) {
		respondWith(t, validBody)
		release, err := GetAddonRelease("addon", "v1.2.0")
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		want := Release{
			ZipballUrl:  "https://example.com/a.zip",
			TagName:     "v1.2.0",
			Body:        "notes",
			PublishedAt: time.Date(2024, 5, 1, 12, 0, 0, 0, time.UTC),
			Tag:         Tag{Ref: "refs/tags/v1.2.0", Sha: "abc", Type: "commit", Url: "https://example.com/tag"},
		}
		if !release.PublishedAt.Equal(want.PublishedAt) {
			t.Fatalf("published_at: got %v, want %v", release.PublishedAt, want.PublishedAt)
		}
		release.PublishedAt = want.PublishedAt
		if release != want {
			t.Fatalf("got %+v, want %+v", release, want)
		}
	})

	for _, tc := range []struct {
		name string
		body string
	}{
		{"missing data", `{"status":true}`},
		{"null data", `{"status":true,"data":null}`},
		{"data is a string", `{"status":true,"data":"oops"}`},
		{"missing release", `{"status":true,"data":{"tag":{}}}`},
		{"release is an array", `{"status":true,"data":{"release":[]}}`},
		{"zipball_url wrong type", `{"status":true,"data":{"release":{"zipball_url":5,"tag_name":"v1","published_at":"2024-05-01T12:00:00Z"}}}`},
		{"missing zipball_url", `{"status":true,"data":{"release":{"tag_name":"v1","published_at":"2024-05-01T12:00:00Z"}}}`},
		{"missing tag_name", `{"status":true,"data":{"release":{"zipball_url":"u","published_at":"2024-05-01T12:00:00Z"}}}`},
		{"missing published_at", `{"status":true,"data":{"release":{"zipball_url":"u","tag_name":"v1"}}}`},
		{"invalid published_at", `{"status":true,"data":{"release":{"zipball_url":"u","tag_name":"v1","published_at":"yesterday"}}}`},
		{"tag wrong type", `{"status":true,"data":{"release":{"zipball_url":"u","tag_name":"v1","published_at":"2024-05-01T12:00:00Z"},"tag":"x"}}`},
	} {
		t.Run(tc.name, func(t *testing.T) {
			respondWith(t, tc.body)
			release, err := GetAddonRelease("addon", "v1")
			if err == nil {
				t.Fatalf("expected error, got release %+v", release)
			}
		})
	}
}

func TestGetLatestApplicationReleaseDecoding(t *testing.T) {
	t.Run("valid payload", func(t *testing.T) {
		respondWith(t, `{"status":true,"data":{"version":"2.0.0","url":"https://example.com/app"}}`)
		release, err := GetLatestApplicationRelease()
		if err != nil {
			t.Fatalf("unexpected error: %v", err)
		}
		want := ApplicationRelease{Version: "2.0.0", Url: "https://example.com/app"}
		if release != want {
			t.Fatalf("got %+v, want %+v", release, want)
		}
	})

	for _, tc := range []struct {
		name string
		body string
	}{
		{"missing data", `{"status":true}`},
		{"null data", `{"status":true,"data":null}`},
		{"data is an array", `{"status":true,"data":[]}`},
		{"version wrong type", `{"status":true,"data":{"version":2,"url":"u"}}`},
		{"missing version", `{"status":true,"data":{"url":"u"}}`},
		{"missing url", `{"status":true,"data":{"version":"2.0.0"}}`},
	} {
		t.Run(tc.name, func(t *testing.T) {
			respondWith(t, tc.body)
			release, err := GetLatestApplicationRelease()
			if err == nil {
				t.Fatalf("expected error, got release %+v", release)
			}
		})
	}
}
