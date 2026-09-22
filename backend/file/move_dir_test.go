package file

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestCopyDirPreservesReadOnlyMode(t *testing.T) {
	src := t.TempDir()
	dest := filepath.Join(t.TempDir(), "dest")

	srcFile := filepath.Join(src, "readonly.txt")
	if err := os.WriteFile(srcFile, []byte("ro"), 0644); err != nil {
		t.Fatalf("seed source file: %v", err)
	}
	if err := os.Chmod(srcFile, 0444); err != nil {
		t.Fatalf("chmod source file: %v", err)
	}

	if err := CopyDir(src, dest); err != nil {
		t.Fatalf("CopyDir: %v", err)
	}

	info, err := os.Stat(filepath.Join(dest, "readonly.txt"))
	if err != nil {
		t.Fatalf("stat copied file: %v", err)
	}
	if got := info.Mode().Perm(); got != 0444 {
		t.Fatalf("copied file mode = %o, want 0444", got)
	}
}

func TestCopyDirPreservesExecutableBit(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("executable bits are not representable on windows")
	}

	src := t.TempDir()
	dest := filepath.Join(t.TempDir(), "dest")

	srcFile := filepath.Join(src, "helper.sh")
	if err := os.WriteFile(srcFile, []byte("#!/bin/sh\n"), 0644); err != nil {
		t.Fatalf("seed source file: %v", err)
	}
	if err := os.Chmod(srcFile, 0755); err != nil {
		t.Fatalf("chmod source file: %v", err)
	}

	if err := CopyDir(src, dest); err != nil {
		t.Fatalf("CopyDir: %v", err)
	}

	info, err := os.Stat(filepath.Join(dest, "helper.sh"))
	if err != nil {
		t.Fatalf("stat copied file: %v", err)
	}
	if got := info.Mode().Perm(); got != 0755 {
		t.Fatalf("copied file mode = %o, want 0755", got)
	}
}
