package updater

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

// A minimal x86-64 ELF header carrying the type 2 AppImage marker.
const appImageFixtureHex = "7f454c4602010100414902000000000002003e000100000000000000000000000000000000000000000000000000000000000000400038000000400000000000"
const appImageFixtureChecksum = "8b37eaa35b5f84ba96af907362f9b9fa9cf5adc31d454475ad99f69573d2c64e"

func appImageFixture(t *testing.T) []byte {
	t.Helper()
	data, err := hex.DecodeString(appImageFixtureHex)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func installAppImageFixture(t *testing.T) (string, []byte) {
	t.Helper()
	data := appImageFixture(t)
	path := filepath.Join(t.TempDir(), "Classic Addon Manager.AppImage")
	if err := os.WriteFile(path, data, 0750); err != nil {
		t.Fatal(err)
	}
	t.Setenv("APPIMAGE", path)
	return path, data
}

func TestAppImageUpdateRequiresChecksumBeforeDownloading(t *testing.T) {
	target, original := installAppImageFixture(t)
	for _, checksum := range []string{"", "sha256:" + appImageFixtureChecksum, "invalid", strings.Repeat("a", 63)} {
		_, err := prepareAppImageUpdate("http://127.0.0.1:1/never-downloaded", checksum)
		if !errors.Is(err, ErrUpdateVerificationFailed) {
			t.Fatalf("checksum %q: expected verification failure before network access, got %v", checksum, err)
		}
	}
	assertAppImageUnchanged(t, target, original)
}

func assertAppImageUnchanged(t *testing.T, target string, original []byte) {
	t.Helper()
	data, err := os.ReadFile(target)
	if err != nil || !bytes.Equal(data, original) {
		t.Fatalf("installed AppImage changed: %v", err)
	}
	entries, err := os.ReadDir(filepath.Dir(target))
	if err != nil {
		t.Fatal(err)
	}
	for _, entry := range entries {
		if strings.HasPrefix(entry.Name(), ".classic-addon-manager-update-") {
			t.Fatalf("failed update left staging data: %s", entry.Name())
		}
	}
}

func TestAppImageUpdateRejectsInvalidDownloadsWithoutChangingInstallation(t *testing.T) {
	valid := appImageFixture(t)
	for _, tc := range []struct {
		name     string
		payload  []byte
		checksum string
		status   int
	}{
		{"checksum mismatch", []byte("corrupt download"), appImageFixtureChecksum, 200},
		{"deb instead of AppImage", []byte("!<arch>\n"), "", 200},
		{"download failed", valid, appImageFixtureChecksum, 503},
	} {
		t.Run(tc.name, func(t *testing.T) {
			target, original := installAppImageFixture(t)
			checksum := tc.checksum
			if checksum == "" {
				checksum = fmt.Sprintf("%x", sha256.Sum256(tc.payload))
			}
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
				w.WriteHeader(tc.status)
				_, _ = w.Write(tc.payload)
			}))
			defer server.Close()
			if _, err := prepareAppImageUpdate(server.URL, checksum); err == nil {
				t.Fatal("expected invalid download to be rejected")
			}
			assertAppImageUnchanged(t, target, original)
		})
	}
}

func TestAppImageUpdateStagesVerifiedImageWithoutReplacingRunningFile(t *testing.T) {
	target, original := installAppImageFixture(t)
	payload := bytes.Clone(original)
	payload[15] = 1 // Change an ELF padding byte without changing its platform or format.
	checksum := fmt.Sprintf("%x", sha256.Sum256(payload))
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write(payload)
	}))
	defer server.Close()

	update, err := prepareAppImageUpdate(server.URL, checksum)
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(update.StagingDir)
	installed, err := os.ReadFile(target)
	if err != nil || !bytes.Equal(installed, original) {
		t.Fatalf("running file was replaced before helper startup: %v", err)
	}
	staged, err := os.ReadFile(update.SourcePath)
	if err != nil || !bytes.Equal(staged, payload) {
		t.Fatalf("verified image was not staged intact: %v", err)
	}
	if filepath.Dir(update.StagingDir) != filepath.Dir(target) {
		t.Fatal("update was staged outside the target filesystem")
	}
	if runtime.GOOS == "linux" {
		info, err := os.Stat(update.SourcePath)
		if err != nil || info.Mode().Perm() != 0750 {
			t.Fatalf("execute permissions were not preserved: %v", err)
		}
	}
}
