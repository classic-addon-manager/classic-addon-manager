package util

import (
	"ClassicAddonManager/backend/api"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestDownloadFileSuccess(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "addon.zip")

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if got := r.Header.Get("X-Client"); got != api.GetClientHeader() {
			t.Errorf("X-Client = %q, want %q", got, api.GetClientHeader())
		}
		if got := r.Header.Get("Accept"); got != "application/octet-stream, */*" {
			t.Errorf("Accept = %q, want %q", got, "application/octet-stream, */*")
		}
		w.Write([]byte("addon payload"))
	}))
	defer server.Close()

	if err := DownloadFile(server.URL, target); err != nil {
		t.Fatalf("DownloadFile: %v", err)
	}
	if got := readTestFile(t, target); got != "addon payload" {
		t.Fatalf("target content = %q, want %q", got, "addon payload")
	}
	if names := dirEntryNames(t, dir); len(names) != 1 || names[0] != "addon.zip" {
		t.Fatalf("dir must contain only the downloaded file, got %v", names)
	}
}

func TestDownloadFileServerError(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "addon.zip")

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer server.Close()

	if err := DownloadFile(server.URL, target); err == nil {
		t.Fatal("expected error for 500 response")
	}
	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Fatal("target must not exist after failed download")
	}
	if names := dirEntryNames(t, dir); len(names) != 0 {
		t.Fatalf("dir must be empty after failed download, got %v", names)
	}
}

func TestDownloadFileTruncatedBody(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "addon.zip")

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Length", "100")
		w.Write([]byte("0123456789"))
	}))
	defer server.Close()

	if err := DownloadFile(server.URL, target); err == nil {
		t.Fatal("expected error for truncated body")
	}
	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Fatal("target must not exist after truncated download")
	}
	if names := dirEntryNames(t, dir); len(names) != 0 {
		t.Fatalf("dir must be empty after truncated download, got %v", names)
	}
}

func TestDownloadFileStallTimeout(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "addon.zip")

	prev := downloadClient
	downloadClient = &http.Client{Timeout: 200 * time.Millisecond}
	t.Cleanup(func() { downloadClient = prev })

	release := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Write([]byte("partial"))
		if f, ok := w.(http.Flusher); ok {
			f.Flush()
		}
		select {
		case <-r.Context().Done():
		case <-release:
		}
	}))
	t.Cleanup(func() {
		close(release)
		server.Close()
	})

	if err := DownloadFile(server.URL, target); err == nil {
		t.Fatal("expected timeout error for stalled download")
	}
	if _, err := os.Stat(target); !os.IsNotExist(err) {
		t.Fatal("target must not exist after timed-out download")
	}
	if names := dirEntryNames(t, dir); len(names) != 0 {
		t.Fatalf("dir must be empty after timed-out download, got %v", names)
	}
}

func TestDownloadFileFailurePreservesExistingTarget(t *testing.T) {
	dir := t.TempDir()
	target := filepath.Join(dir, "addon.zip")
	writeTestFile(t, target, "old")

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Length", "100")
		w.Write([]byte("0123456789"))
	}))
	defer server.Close()

	if err := DownloadFile(server.URL, target); err == nil {
		t.Fatal("expected error for truncated body")
	}
	if got := readTestFile(t, target); got != "old" {
		t.Fatalf("target content = %q, want preserved %q", got, "old")
	}
	if names := dirEntryNames(t, dir); len(names) != 1 || names[0] != "addon.zip" {
		t.Fatalf("dir must contain only the original file, got %v", names)
	}
}
