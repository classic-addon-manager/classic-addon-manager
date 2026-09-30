//go:build linux

package updater

import (
	"bytes"
	"context"
	"crypto/sha256"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

func TestAppImageUpdateRejectsReadOnlyDirectory(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root bypasses directory write permissions")
	}
	setSelfUpdateOS(t, "linux")
	target, original := installAppImageFixture(t)
	dir := filepath.Dir(target)
	if err := os.Chmod(dir, 0555); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(dir, 0755)
	if SelfUpdateSupported() {
		t.Fatal("read-only installation offered automatic update")
	}
	if err := SelfUpdate("http://127.0.0.1:1/never-downloaded", appImageFixtureChecksum); err == nil {
		t.Fatal("read-only installation accepted an update")
	}
	assertAppImageUnchanged(t, target, original)
}

func TestAppImageUpdateHelperStartFailureLeavesOriginalUsable(t *testing.T) {
	setSelfUpdateOS(t, "linux")
	target, original := installAppImageFixture(t)
	t.Setenv("XDG_CACHE_HOME", t.TempDir())
	t.Setenv("OWD", filepath.Join(t.TempDir(), "missing-working-directory"))
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write(original)
	}))
	defer server.Close()
	if err := SelfUpdate(server.URL, appImageFixtureChecksum); err == nil {
		t.Fatal("expected helper startup failure")
	}
	assertAppImageUnchanged(t, target, original)
}

func TestAppImageUpdatePreservesReadOnlyExecutablePermissions(t *testing.T) {
	target, payload := installAppImageFixture(t)
	if err := os.Chmod(target, 0555); err != nil {
		t.Fatal(err)
	}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write(payload)
	}))
	defer server.Close()
	update, err := prepareAppImageUpdate(server.URL, appImageFixtureChecksum)
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(update.StagingDir)
	info, err := os.Stat(update.SourcePath)
	if err != nil || info.Mode().Perm() != 0555 {
		t.Fatalf("read-only executable permissions were lost: %v", err)
	}
}

func TestAppImageReplacementFailureKeepsOriginalAndReportsRecoveryPath(t *testing.T) {
	target, original := installAppImageFixture(t)
	staging := t.TempDir()
	source := filepath.Join(staging, "missing.AppImage")
	failure := filepath.Join(t.TempDir(), updateFailureFileName)
	script := filepath.Join(staging, "update.sh")
	if err := os.WriteFile(script, []byte(appImageUpdateScript), 0600); err != nil {
		t.Fatal(err)
	}
	// A reaped child gives the helper a parent that has already exited.
	parent := exec.Command("/bin/true")
	if err := parent.Run(); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command("/bin/sh", script, strconv.Itoa(parent.Process.Pid), target, source, staging, failure)
	if err := cmd.Run(); err == nil {
		t.Fatal("replacement failure was reported as success")
	}
	assertAppImageUnchanged(t, target, original)
	report, err := consumeUpdateFailure(failure)
	if err != nil || !strings.Contains(report, target) || !strings.Contains(report, source) {
		t.Fatalf("failure report did not identify the installation and recovery download: %q, %v", report, err)
	}
}

// Re-execute a real ELF process with an AppImage marker to exercise download,
// verification, parent exit, atomic replacement and relaunch without a GUI/FUSE mount.
func TestAppImageUpdateRestartsAfterOldProcessExits(t *testing.T) {
	if os.Getenv("CAM_UPDATE_PROCESS_TEST") == "1" {
		pidPath := os.Getenv("CAM_UPDATE_PID_PATH")
		if os.Getenv("APPIMAGE") != "" {
			if err := os.WriteFile(pidPath, []byte(strconv.Itoa(os.Getpid())), 0600); err != nil {
				t.Fatal(err)
			}
			if err := SelfUpdate(os.Getenv("CAM_UPDATE_URL"), os.Getenv("CAM_UPDATE_CHECKSUM")); err != nil {
				t.Fatal(err)
			}
			t.Fatal("successful update should have exited the old process")
		}
		pid, err := os.ReadFile(pidPath)
		if err != nil {
			t.Fatal(err)
		}
		result := "restarted after exit"
		if _, err := os.Stat("/proc/" + string(pid)); !os.IsNotExist(err) {
			result = "old process still exists"
		}
		for _, key := range []string{"APPDIR", "ARGV0", "OWD"} {
			if os.Getenv(key) != "" {
				result = "stale AppImage environment: " + key
			}
		}
		if strings.Contains(os.Getenv("LD_LIBRARY_PATH"), ".mount_CAM") || strings.Contains(os.Getenv("PATH"), ".mount_CAM") {
			result = "mounted libraries or tools leaked into relaunch"
		}
		conn, err := net.Dial("unix", os.Getenv("CAM_UPDATE_SOCKET"))
		if err != nil {
			t.Fatal(err)
		}
		_, _ = io.WriteString(conn, result)
		conn.Close()
		os.Exit(0)
	}

	testExe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	payload, err := os.ReadFile(testExe)
	if err != nil {
		t.Fatal(err)
	}
	copy(payload[8:11], "AI\x02")
	checksum := fmt.Sprintf("%x", sha256.Sum256(payload))
	targetDir := t.TempDir()
	target := filepath.Join(targetDir, "it's $HOME; Classic Addon Manager.AppImage")
	if err := os.WriteFile(target, appImageFixture(t), 0750); err != nil {
		t.Fatal(err)
	}
	launcher := filepath.Join(targetDir, "current.AppImage")
	if err := os.Symlink(target, launcher); err != nil {
		t.Fatal(err)
	}
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		_, _ = w.Write(payload)
	}))
	defer server.Close()

	stateDir := t.TempDir()
	socket := filepath.Join(stateDir, "relaunch.sock")
	listener, err := net.ListenUnix("unix", &net.UnixAddr{Name: socket, Net: "unix"})
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()
	_ = listener.SetDeadline(time.Now().Add(30 * time.Second))
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, testExe, "-test.run=^TestAppImageUpdateRestartsAfterOldProcessExits$")
	cmd.Env = append(os.Environ(),
		"CAM_UPDATE_PROCESS_TEST=1", "APPIMAGE="+launcher,
		"APPDIR=/tmp/.mount_CAM", "ARGV0="+launcher, "OWD="+targetDir,
		"LD_LIBRARY_PATH=/tmp/.mount_CAM/usr/lib", "PATH=/tmp/.mount_CAM/usr/bin:/usr/bin:/bin",
		"XDG_CACHE_HOME="+filepath.Join(stateDir, "cache"),
		"CAM_UPDATE_PID_PATH="+filepath.Join(stateDir, "old.pid"), "CAM_UPDATE_SOCKET="+socket,
		"CAM_UPDATE_URL="+server.URL, "CAM_UPDATE_CHECKSUM="+checksum,
	)
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("updating process failed: %v\n%s", err, output)
	}
	conn, err := listener.AcceptUnix()
	if err != nil {
		t.Fatalf("new version did not relaunch: %v", err)
	}
	_ = conn.SetReadDeadline(time.Now().Add(5 * time.Second))
	result, err := io.ReadAll(conn)
	conn.Close()
	if err != nil || string(result) != "restarted after exit" {
		t.Fatalf("relaunch result: %q, %v", result, err)
	}
	installed, err := os.ReadFile(target)
	if err != nil || !bytes.Equal(installed, payload) {
		t.Fatalf("replacement was not installed intact: %v", err)
	}
	info, err := os.Stat(target)
	if err != nil || info.Mode().Perm() != 0750 {
		t.Fatalf("replacement lost execute permissions: %v", err)
	}
	link, err := os.Readlink(launcher)
	if err != nil || link != target {
		t.Fatalf("launcher symlink was replaced: %v", err)
	}
	entries, err := os.ReadDir(targetDir)
	if err != nil {
		t.Fatal(err)
	}
	for _, entry := range entries {
		if entry.Name() != filepath.Base(target) && entry.Name() != filepath.Base(launcher) {
			t.Fatalf("successful update retained a backup or staging data: %s", entry.Name())
		}
	}
}
