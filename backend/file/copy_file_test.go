package file

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

func TestCopyFileKeepsSourceAndReplacesDestination(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source.txt")
	dest := filepath.Join(dir, "destination.txt")
	if err := os.WriteFile(src, []byte("copy me"), 0600); err != nil {
		t.Fatalf("create source: %v", err)
	}
	modified := time.Date(2020, 1, 2, 3, 4, 5, 0, time.UTC)
	if err := os.Chtimes(src, modified, modified); err != nil {
		t.Fatalf("set source time: %v", err)
	}
	if err := os.WriteFile(dest, []byte("previous, longer contents"), 0644); err != nil {
		t.Fatalf("create destination: %v", err)
	}

	if err := CopyFile(src, dest); err != nil {
		t.Fatalf("CopyFile: %v", err)
	}
	for _, path := range []string{src, dest} {
		data, err := os.ReadFile(path)
		if err != nil {
			t.Fatalf("read %s: %v", path, err)
		}
		if string(data) != "copy me" {
			t.Fatalf("contents of %s = %q", path, data)
		}
	}
	info, err := os.Stat(dest)
	if err != nil {
		t.Fatalf("stat destination: %v", err)
	}
	if !info.ModTime().Equal(modified) {
		t.Fatalf("destination modified time = %v, want %v", info.ModTime(), modified)
	}
	if runtime.GOOS != "windows" && info.Mode().Perm() != 0600 {
		t.Fatalf("destination mode = %o, want 0600", info.Mode().Perm())
	}
}
