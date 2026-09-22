//go:build linux

package file

import (
	"os"
	"path/filepath"
	"syscall"
	"testing"
)

func TestWriteAtomicNewFileRespectsUmask(t *testing.T) {
	old := syscall.Umask(0077)
	defer syscall.Umask(old)

	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")

	if err := WriteAtomic(path, []byte("x"), 0644); err != nil {
		t.Fatalf("WriteAtomic: %v", err)
	}

	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("stat written file: %v", err)
	}
	if got := info.Mode().Perm(); got != 0600 {
		t.Fatalf("expected mode 0600, got %o", got)
	}
}

func TestWriteAtomicPreservesExistingMode(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")

	if err := os.WriteFile(path, []byte("old"), 0644); err != nil {
		t.Fatalf("seed file: %v", err)
	}
	if err := os.Chmod(path, 0640); err != nil {
		t.Fatalf("chmod seed file: %v", err)
	}

	old := syscall.Umask(0077)
	defer syscall.Umask(old)

	if err := WriteAtomic(path, []byte("new"), 0644); err != nil {
		t.Fatalf("WriteAtomic: %v", err)
	}

	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("stat written file: %v", err)
	}
	if got := info.Mode().Perm(); got != 0640 {
		t.Fatalf("expected mode 0640, got %o", got)
	}
}
