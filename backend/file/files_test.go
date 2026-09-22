package file

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func assertNoTempFiles(t *testing.T, dir, base string) {
	t.Helper()
	matches, err := filepath.Glob(filepath.Join(dir, "."+base+".tmp-*"))
	if err != nil {
		t.Fatalf("glob temp files: %v", err)
	}
	if len(matches) != 0 {
		t.Fatalf("found leftover temp files: %v", matches)
	}
}

func TestWriteAtomicCreatesFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")

	if err := WriteAtomic(path, []byte(`{"a":1}`), 0644); err != nil {
		t.Fatalf("WriteAtomic: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read written file: %v", err)
	}
	if string(data) != `{"a":1}` {
		t.Fatalf("unexpected contents: %q", data)
	}
	assertNoTempFiles(t, dir, "state.json")
}

func TestWriteAtomicReplacesExistingFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")

	if err := os.WriteFile(path, []byte("old contents that are longer"), 0644); err != nil {
		t.Fatalf("seed file: %v", err)
	}

	if err := WriteAtomic(path, []byte("new"), 0644); err != nil {
		t.Fatalf("WriteAtomic: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read written file: %v", err)
	}
	if string(data) != "new" {
		t.Fatalf("unexpected contents: %q", data)
	}
	assertNoTempFiles(t, dir, "state.json")
}

func TestWriteAtomicCreatesParentDirs(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "nested", "deeper", "state.json")

	if err := WriteAtomic(path, []byte("x"), 0644); err != nil {
		t.Fatalf("WriteAtomic: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read written file: %v", err)
	}
	if string(data) != "x" {
		t.Fatalf("unexpected contents: %q", data)
	}
}

func TestWriteAtomicPermissions(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("permission bits not enforced on windows")
	}

	dir := t.TempDir()
	path := filepath.Join(dir, "secret.json")

	if err := WriteAtomic(path, []byte("x"), 0600); err != nil {
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

func TestWriteLinesJoinsWithSingleNewline(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "addons.txt")

	if err := WriteLines(path, []string{"aaa", "bbb", "ccc"}); err != nil {
		t.Fatalf("WriteLines: %v", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read written file: %v", err)
	}
	if string(data) != "aaa\nbbb\nccc" {
		t.Fatalf("unexpected contents: %q", data)
	}
	assertNoTempFiles(t, dir, "addons.txt")
}
