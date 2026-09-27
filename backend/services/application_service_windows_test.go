//go:build windows

package services

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

type updateFixture struct {
	source, target, failure, script string
}

func newUpdateFixture(t *testing.T, attempts int) updateFixture {
	t.Helper()
	dir := t.TempDir()
	fixture := updateFixture{
		source:  filepath.Join(dir, "new 100%.exe"),
		target:  filepath.Join(dir, "app.exe"),
		failure: filepath.Join(dir, updateFailureFileName),
		script:  filepath.Join(dir, "update.bat"),
	}
	writeFile(t, fixture.source, "new")
	writeFile(t, fixture.target, "old")
	script := buildWindowsUpdateScript(windowsUpdateScript{
		NewExePath:  fixture.source,
		TargetPath:  fixture.target,
		FailurePath: fixture.failure,
		Attempts:    attempts,
	})
	writeFile(t, fixture.script, script)
	return fixture
}

func writeFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.WriteFile(path, []byte(content), 0644); err != nil {
		t.Fatal(err)
	}
}

// lockFile opens the file without sharing, the same way a running executable blocks being overwritten.
func lockFile(t *testing.T, path string) syscall.Handle {
	t.Helper()
	name, err := syscall.UTF16PtrFromString(path)
	if err != nil {
		t.Fatal(err)
	}
	handle, err := syscall.CreateFile(name, syscall.GENERIC_READ, 0, nil, syscall.OPEN_EXISTING, syscall.FILE_ATTRIBUTE_NORMAL, 0)
	if err != nil {
		t.Fatal(err)
	}
	return handle
}

func runScript(path string) error {
	return exec.Command("cmd", "/C", path).Run()
}

func TestWindowsUpdateScriptRetriesUntilTargetIsReleased(t *testing.T) {
	fixture := newUpdateFixture(t, 30)
	handle := lockFile(t, fixture.target)

	done := make(chan error, 1)
	go func() { done <- runScript(fixture.script) }()

	time.Sleep(1500 * time.Millisecond)
	syscall.CloseHandle(handle)

	select {
	case err := <-done:
		if err != nil {
			t.Fatalf("script failed: %v", err)
		}
	case <-time.After(30 * time.Second):
		t.Fatal("script did not finish")
	}

	if data, _ := os.ReadFile(fixture.target); string(data) != "new" {
		t.Fatalf("expected target to be replaced, got %q", data)
	}
	for _, leftover := range []string{fixture.source, fixture.script, fixture.failure} {
		if _, err := os.Stat(leftover); !os.IsNotExist(err) {
			t.Fatalf("expected %s to be removed after success", leftover)
		}
	}
}

func TestWindowsUpdateScriptKeepsFilesAndReportsWhenReplacementFails(t *testing.T) {
	fixture := newUpdateFixture(t, 2)
	handle := lockFile(t, fixture.target)

	err := runScript(fixture.script)
	syscall.CloseHandle(handle)
	if err == nil {
		t.Fatal("expected script to exit with an error")
	}

	if data, _ := os.ReadFile(fixture.target); string(data) != "old" {
		t.Fatalf("expected target to be untouched, got %q", data)
	}
	if _, err := os.Stat(fixture.source); err != nil {
		t.Fatalf("expected downloaded update to be kept: %v", err)
	}
	if _, err := os.Stat(fixture.script); err != nil {
		t.Fatalf("expected update script to be kept: %v", err)
	}

	report, err := consumeUpdateFailure(fixture.failure)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(report, fixture.target) || !strings.Contains(report, fixture.source) {
		t.Fatalf("expected report to mention both paths, got %q", report)
	}
}
