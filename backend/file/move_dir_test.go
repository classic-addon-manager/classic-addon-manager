package file

import (
	"bytes"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestCopyDirCopiesLargeFile(t *testing.T) {
	src := t.TempDir()
	dest := filepath.Join(t.TempDir(), "dest")
	srcFile, err := os.Create(filepath.Join(src, "large.bin"))
	if err != nil {
		t.Fatalf("create source file: %v", err)
	}
	chunk := bytes.Repeat([]byte("large copy test"), 2048)
	for range 1024 {
		if _, err := srcFile.Write(chunk); err != nil {
			srcFile.Close()
			t.Fatalf("write source file: %v", err)
		}
	}
	if err := srcFile.Close(); err != nil {
		t.Fatalf("close source file: %v", err)
	}

	if err := CopyDir(src, dest); err != nil {
		t.Fatalf("CopyDir: %v", err)
	}
	copyFile, err := os.Open(filepath.Join(dest, "large.bin"))
	if err != nil {
		t.Fatalf("open copied file: %v", err)
	}
	defer copyFile.Close()
	got := make([]byte, len(chunk))
	for range 1024 {
		if _, err := io.ReadFull(copyFile, got); err != nil {
			t.Fatalf("read copied file: %v", err)
		}
		if !bytes.Equal(got, chunk) {
			t.Fatal("copied file contents differ")
		}
	}
	var extra [1]byte
	if _, err := copyFile.Read(extra[:]); err != io.EOF {
		t.Fatalf("expected end of copied file, got %v", err)
	}
}

func TestCopyDirPreservesDirectoryMode(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("directory permission bits are not enforced on windows")
	}
	src := t.TempDir()
	nested := filepath.Join(src, "nested")
	if err := os.Mkdir(nested, 0750); err != nil {
		t.Fatalf("create nested directory: %v", err)
	}
	dest := filepath.Join(t.TempDir(), "dest")
	if err := CopyDir(src, dest); err != nil {
		t.Fatalf("CopyDir: %v", err)
	}
	info, err := os.Stat(filepath.Join(dest, "nested"))
	if err != nil {
		t.Fatalf("stat copied directory: %v", err)
	}
	if got := info.Mode().Perm(); got != 0750 {
		t.Fatalf("copied directory mode = %o, want 0750", got)
	}
}

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
