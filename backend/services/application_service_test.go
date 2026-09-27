package services

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
)

func setSelfUpdateOS(t *testing.T, goos string) {
	t.Helper()
	orig := selfUpdateOS
	selfUpdateOS = goos
	t.Cleanup(func() { selfUpdateOS = orig })
}

func TestSelfUpdateUnsupportedPlatform(t *testing.T) {
	setSelfUpdateOS(t, "linux")
	service := &ApplicationService{}

	if service.SelfUpdateSupported() {
		t.Fatal("expected self-update to be unsupported on linux")
	}
	if err := service.SelfUpdate("http://127.0.0.1:1/never-downloaded"); !errors.Is(err, ErrSelfUpdateUnsupported) {
		t.Fatalf("expected ErrSelfUpdateUnsupported, got %v", err)
	}
}

func TestSelfUpdateSupportedOnWindows(t *testing.T) {
	setSelfUpdateOS(t, "windows")
	if !(&ApplicationService{}).SelfUpdateSupported() {
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
