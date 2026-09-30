package updater

import (
	"errors"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func setSelfUpdateOS(t *testing.T, goos string) {
	t.Helper()
	orig := selfUpdateOS
	selfUpdateOS = goos
	t.Cleanup(func() { selfUpdateOS = orig })
}

func TestSelfUpdateRejectsNonAppImageInstallation(t *testing.T) {
	setSelfUpdateOS(t, "linux")
	t.Setenv("APPIMAGE", "")

	if SelfUpdateSupported() {
		t.Fatal("expected self-update to be unsupported without an AppImage")
	}
	if err := SelfUpdate("http://127.0.0.1:1/never-downloaded", ""); !errors.Is(err, ErrSelfUpdateUnsupported) {
		t.Fatalf("expected ErrSelfUpdateUnsupported, got %v", err)
	}
}

func TestSelfUpdateSupportedOnWindows(t *testing.T) {
	setSelfUpdateOS(t, "windows")
	if !SelfUpdateSupported() {
		t.Fatal("expected self-update to be supported on windows")
	}
}

func TestConsumeUpdateFailureReturnsReportOnce(t *testing.T) {
	path := filepath.Join(t.TempDir(), updateFailureFileName)

	report, err := consumeUpdateFailure(path)
	if err != nil || report != "" {
		t.Fatalf("expected no report when file is missing, got %q, %v", report, err)
	}

	if err := os.WriteFile(path, []byte("copy failed\r\n"), 0644); err != nil {
		t.Fatal(err)
	}
	report, err = consumeUpdateFailure(path)
	if err != nil || report != "copy failed" {
		t.Fatalf("expected trimmed report, got %q, %v", report, err)
	}

	report, err = consumeUpdateFailure(path)
	if err != nil || report != "" {
		t.Fatalf("expected report to be consumed, got %q, %v", report, err)
	}
}

func writeUpdateFile(t *testing.T) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "ClassicAddonManager.new.exe")
	if err := os.WriteFile(path, []byte("update contents"), 0644); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestVerifyUpdateChecksum(t *testing.T) {
	const lower = "3202974203720d579673be047ffeac10c8f2aa7eb9142684f4350a1dfa19b2dc"

	for _, tc := range []struct {
		name     string
		checksum string
	}{
		{"lowercase match", lower},
		{"uppercase match", strings.ToUpper(lower)},
		{"empty checksum skips verification", ""},
	} {
		t.Run(tc.name, func(t *testing.T) {
			path := writeUpdateFile(t)
			if err := verifyUpdateChecksum(path, tc.checksum); err != nil {
				t.Fatalf("expected update to be accepted, got %v", err)
			}
			if _, err := os.Stat(path); err != nil {
				t.Fatalf("expected update file to be kept, got %v", err)
			}
		})
	}

	t.Run("mismatch rejects and deletes the file", func(t *testing.T) {
		path := writeUpdateFile(t)
		wrong := strings.Repeat("0", 64)
		if err := verifyUpdateChecksum(path, wrong); !errors.Is(err, ErrUpdateVerificationFailed) {
			t.Fatalf("expected ErrUpdateVerificationFailed, got %v", err)
		}
		if _, err := os.Stat(path); !errors.Is(err, os.ErrNotExist) {
			t.Fatalf("expected rejected update file to be deleted, got %v", err)
		}
	})
}

func TestSelfUpdateRejectsDownloadedFileWithWrongChecksum(t *testing.T) {
	setSelfUpdateOS(t, "windows")
	tempRoot := t.TempDir()
	t.Setenv("TMPDIR", tempRoot)
	t.Setenv("TMP", tempRoot)
	t.Setenv("TEMP", tempRoot)
	tmpDir := updateTempDir()
	if tmpDir != filepath.Join(tempRoot, "ClassicAddonManager") {
		t.Fatalf("update temp dir = %q, want a directory under %q", tmpDir, tempRoot)
	}
	if err := os.MkdirAll(tmpDir, 0755); err != nil {
		t.Fatal(err)
	}
	// If verification is bypassed, script creation must fail before SelfUpdate can launch it or exit.
	if err := os.Mkdir(filepath.Join(tmpDir, "update.bat"), 0755); err != nil {
		t.Fatal(err)
	}

	requested := make(chan struct{}, 1)
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		requested <- struct{}{}
		_, _ = w.Write([]byte("update contents"))
	}))
	defer server.Close()

	err := SelfUpdate(server.URL, strings.Repeat("0", 64))
	if !errors.Is(err, ErrUpdateVerificationFailed) {
		t.Fatalf("expected ErrUpdateVerificationFailed, got %v", err)
	}
	select {
	case <-requested:
	default:
		t.Fatal("expected the update to be downloaded")
	}
	if _, err := os.Stat(filepath.Join(tmpDir, "ClassicAddonManager.new.exe")); !errors.Is(err, os.ErrNotExist) {
		t.Fatalf("expected rejected update file to be deleted, got %v", err)
	}
}
