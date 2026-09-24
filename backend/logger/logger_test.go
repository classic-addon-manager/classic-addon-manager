package logger

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"runtime/debug"
	"strings"
	"sync"
	"testing"
	"time"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"go.uber.org/zap/zaptest/observer"
)

func TestConcurrentUse(t *testing.T) {
	// The singleton opens app.log on first use and never closes it, so point
	// the config dir at the shared temp dir rather than t.TempDir() (Windows
	// cannot remove a temp dir holding an open file).
	t.Setenv("APPDATA", os.TempDir())
	t.Setenv("XDG_CONFIG_HOME", os.TempDir())

	var wg sync.WaitGroup
	for i := 0; i < 50; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for j := 0; j < 100; j++ {
				Info("info")
				Warn("warn")
				Error("error")
				Error("error", errors.New("x"))
				Sync()
			}
		}()
	}
	wg.Wait()
}

func TestErrorField(t *testing.T) {
	core, logs := observer.New(zapcore.ErrorLevel)
	l := zap.New(core).Sugar()

	logError(l, "m")
	logError(l, "m", nil)
	logError(l, "m", errors.New("x"))

	entries := logs.All()
	if len(entries) != 3 {
		t.Fatalf("expected 3 entries, got %d", len(entries))
	}
	for i, e := range entries[:2] {
		if _, ok := e.ContextMap()["error"]; ok {
			t.Fatalf("entry %d unexpectedly has an error field", i)
		}
	}
	if got := fmt.Sprint(entries[2].ContextMap()["error"]); got != "x" {
		t.Fatalf("error field = %q, want %q", got, "x")
	}
}

func TestLogPathError(t *testing.T) {
	switch runtime.GOOS {
	case "windows":
		t.Setenv("APPDATA", "")
	default:
		t.Setenv("XDG_CONFIG_HOME", "")
		t.Setenv("HOME", "")
	}
	if _, err := LogPath(); err == nil {
		t.Fatal("LogPath returned nil error")
	}
}

func TestHasBuildTag(t *testing.T) {
	for _, tc := range []struct {
		name     string
		settings []debug.BuildSetting
		want     bool
	}{
		{"single production tag", []debug.BuildSetting{{Key: "-tags", Value: "production"}}, true},
		{"production among several", []debug.BuildSetting{{Key: "-tags", Value: "foo,production"}}, true},
		{"prefix only", []debug.BuildSetting{{Key: "-tags", Value: "productionx"}}, false},
		{"no tags key", []debug.BuildSetting{{Key: "-trimpath", Value: "true"}}, false},
		{"no settings", nil, false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			if got := hasBuildTag(tc.settings, "production"); got != tc.want {
				t.Fatalf("hasBuildTag(%v) = %v, want %v", tc.settings, got, tc.want)
			}
		})
	}
}

func TestNewFileLoggerOpenFailure(t *testing.T) {
	// The NUL byte makes Stat fail with EINVAL (not IsNotExist) after MkdirAll
	// on the parent has already succeeded, so lumberjack's open errors out.
	l, closer, err := newFileLogger(filepath.Join(t.TempDir(), "app\x00.log"), 1, 1, nil)
	if err == nil {
		t.Fatal("expected error for unopenable log path")
	}
	if !strings.Contains(err.Error(), "open log file") {
		t.Fatalf("error = %v, want it to mention open log file", err)
	}
	if l != nil || closer != nil {
		t.Fatalf("expected nil logger and closer, got %v, %v", l, closer)
	}
}

func TestRotation(t *testing.T) {
	dir := t.TempDir()
	l, closer, err := newFileLogger(filepath.Join(dir, "app.log"), 1, 1, nil)
	if err != nil {
		t.Fatalf("newFileLogger: %v", err)
	}

	line := strings.Repeat("x", 1024)
	for i := 0; i < 3000; i++ {
		l.Info(line)
	}
	_ = l.Sync()
	if err := closer.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}

	// lumberjack prunes old backups asynchronously, so poll until it settles.
	deadline := time.Now().Add(2 * time.Second)
	var entries []os.DirEntry
	for {
		entries, err = os.ReadDir(dir)
		if err != nil {
			t.Fatalf("readdir: %v", err)
		}
		if len(entries) <= 2 {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("expected at most 2 files after rotation, got %d", len(entries))
		}
		time.Sleep(50 * time.Millisecond)
	}

	// Rotation happens when a write would exceed MaxSize, so a file may hold
	// at most MaxSize plus one encoded line.
	maxFileSize := int64(1<<20) + int64(len(line)) + 512
	for _, e := range entries {
		info, err := e.Info()
		if err != nil {
			t.Fatalf("stat %s: %v", e.Name(), err)
		}
		if info.Size() > maxFileSize {
			t.Fatalf("%s is %d bytes, exceeds MaxSize plus one line", e.Name(), info.Size())
		}
	}
}

func TestConsoleTee(t *testing.T) {
	dir := t.TempDir()
	var buf bytes.Buffer
	l, closer, err := newFileLogger(filepath.Join(dir, "app.log"), 1, 1, zapcore.AddSync(&buf))
	if err != nil {
		t.Fatalf("newFileLogger: %v", err)
	}
	if _, err := os.Stat(filepath.Join(dir, "app.log")); err != nil {
		t.Fatalf("app.log not created eagerly: %v", err)
	}

	l.Info("hello")
	l.Errorw("boom", "error", errors.New("x"))
	_ = l.Sync()
	if err := closer.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}

	console := buf.String()
	for _, want := range []string{"INFO", "hello", "ERROR", "boom", `"error": "x"`} {
		if !strings.Contains(console, want) {
			t.Fatalf("console output missing %q:\n%s", want, console)
		}
	}
	for _, line := range strings.Split(strings.TrimSpace(console), "\n") {
		if strings.HasPrefix(line, "{") {
			t.Fatalf("console line looks like JSON: %q", line)
		}
	}

	data, err := os.ReadFile(filepath.Join(dir, "app.log"))
	if err != nil {
		t.Fatalf("read app.log: %v", err)
	}
	lines := strings.Split(strings.TrimSpace(string(data)), "\n")
	if len(lines) != 2 {
		t.Fatalf("expected 2 lines in app.log, got %d", len(lines))
	}
	for i, want := range []string{"hello", "boom"} {
		var entry map[string]any
		if err := json.Unmarshal([]byte(lines[i]), &entry); err != nil {
			t.Fatalf("app.log line %d is not JSON: %v", i, err)
		}
		if entry["msg"] != want {
			t.Fatalf("app.log line %d msg = %v, want %q", i, entry["msg"], want)
		}
	}
}

func TestAppendLoggerRotatesAtStartup(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "app.log")
	big := strings.Repeat("a", 200)
	if err := os.WriteFile(path, []byte(big), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path+".1", []byte("old"), 0600); err != nil {
		t.Fatal(err)
	}

	l, closer, err := newAppendLogger(path, 100, nil)
	if err != nil {
		t.Fatalf("newAppendLogger: %v", err)
	}
	l.Info("fresh")
	_ = l.Sync()
	if err := closer.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}

	rotated, err := os.ReadFile(path + ".1")
	if err != nil {
		t.Fatalf("read rotated file: %v", err)
	}
	if string(rotated) != big {
		t.Fatalf("rotated file = %q, want the 200-byte previous log", rotated)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read log: %v", err)
	}
	lines := strings.Split(strings.TrimSpace(string(data)), "\n")
	if len(lines) != 1 {
		t.Fatalf("expected 1 line, got %d", len(lines))
	}
	var entry map[string]any
	if err := json.Unmarshal([]byte(lines[0]), &entry); err != nil {
		t.Fatalf("line is not JSON: %v", err)
	}
	if entry["msg"] != "fresh" {
		t.Fatalf("msg = %v, want %q", entry["msg"], "fresh")
	}
}

func TestAppendLoggerKeepsSmallFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "app.log")
	if err := os.WriteFile(path, []byte("{\"msg\":\"prior\"}\n"), 0600); err != nil {
		t.Fatal(err)
	}

	l, closer, err := newAppendLogger(path, 1<<20, nil)
	if err != nil {
		t.Fatalf("newAppendLogger: %v", err)
	}
	l.Info("next")
	_ = l.Sync()
	if err := closer.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read log: %v", err)
	}
	lines := strings.Split(strings.TrimSpace(string(data)), "\n")
	if len(lines) != 2 {
		t.Fatalf("expected 2 lines, got %d", len(lines))
	}
	for i, want := range []string{"prior", "next"} {
		var entry map[string]any
		if err := json.Unmarshal([]byte(lines[i]), &entry); err != nil {
			t.Fatalf("line %d is not JSON: %v", i, err)
		}
		if entry["msg"] != want {
			t.Fatalf("line %d msg = %v, want %q", i, entry["msg"], want)
		}
	}
	if _, err := os.Stat(path + ".1"); !os.IsNotExist(err) {
		t.Fatalf("unexpected rotated file, stat err = %v", err)
	}
}

func TestAppendLoggerConcurrentWriters(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "app.log")

	l1, c1, err := newAppendLogger(path, 1<<20, nil)
	if err != nil {
		t.Fatalf("newAppendLogger: %v", err)
	}
	l2, c2, err := newAppendLogger(path, 1<<20, nil)
	if err != nil {
		t.Fatalf("newAppendLogger: %v", err)
	}

	var wg sync.WaitGroup
	for _, l := range []*zap.SugaredLogger{l1, l2} {
		wg.Add(1)
		go func(l *zap.SugaredLogger) {
			defer wg.Done()
			for i := 0; i < 500; i++ {
				l.Info("line")
			}
		}(l)
	}
	wg.Wait()
	_ = l1.Sync()
	_ = l2.Sync()
	if err := c1.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}
	if err := c2.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read log: %v", err)
	}
	lines := strings.Split(strings.TrimSpace(string(data)), "\n")
	if len(lines) != 1000 {
		t.Fatalf("expected 1000 lines, got %d", len(lines))
	}
	for i, line := range lines {
		var entry map[string]any
		if err := json.Unmarshal([]byte(line), &entry); err != nil {
			t.Fatalf("line %d is not JSON: %v", i, err)
		}
	}
}

func TestOpenLoggerHeadlessPath(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("APPDATA", dir)
	t.Setenv("XDG_CONFIG_HOME", dir)
	logDir := filepath.Join(dir, "ClassicAddonManager")

	l, closer := openLogger(true)
	if closer == nil {
		t.Fatal("openLogger(true) fell back to stderr")
	}
	l.Info("headless")
	_ = l.Sync()
	if err := closer.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}

	info, err := os.Stat(filepath.Join(logDir, "check-updates.log"))
	if err != nil {
		t.Fatalf("check-updates.log missing: %v", err)
	}
	if info.Size() == 0 {
		t.Fatal("check-updates.log is empty")
	}
	if _, err := os.Stat(filepath.Join(logDir, "app.log")); !os.IsNotExist(err) {
		t.Fatalf("app.log should not exist, stat err = %v", err)
	}

	l2, closer2 := openLogger(false)
	if closer2 == nil {
		t.Fatal("openLogger(false) fell back to stderr")
	}
	l2.Info("gui")
	_ = l2.Sync()
	if err := closer2.Close(); err != nil {
		t.Fatalf("close: %v", err)
	}
	if _, err := os.Stat(filepath.Join(logDir, "app.log")); err != nil {
		t.Fatalf("app.log missing: %v", err)
	}
}
