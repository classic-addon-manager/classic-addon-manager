package file

import (
	"archive/zip"
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

func writeTestZip(t *testing.T, path string, files map[string]string) {
	t.Helper()
	f, err := os.Create(path)
	if err != nil {
		t.Fatalf("create zip %s: %v", path, err)
	}
	w := zip.NewWriter(f)
	for name, content := range files {
		fw, err := w.Create(name)
		if err != nil {
			t.Fatalf("create zip entry %s: %v", name, err)
		}
		if _, err := fw.Write([]byte(content)); err != nil {
			t.Fatalf("write zip entry %s: %v", name, err)
		}
	}
	if err := w.Close(); err != nil {
		t.Fatalf("close zip writer: %v", err)
	}
	if err := f.Close(); err != nil {
		t.Fatalf("close zip file: %v", err)
	}
}

func TestValidateAddonZip(t *testing.T) {
	tests := []struct {
		name    string
		files   map[string]string
		wantErr bool
	}{
		{
			name:    "main.lua at release root",
			files:   map[string]string{"root/main.lua": "x"},
			wantErr: false,
		},
		{
			name: "main.lua at release root with nested extras",
			files: map[string]string{
				"root/main.lua":       "x",
				"root/lib/helper.lua": "x",
				"root/deep/main.lua":  "x",
			},
			wantErr: false,
		},
		{
			name:    "main.lua at archive root only",
			files:   map[string]string{"main.lua": "x"},
			wantErr: true,
		},
		{
			name:    "suffix match only",
			files:   map[string]string{"root/xmain.lua": "x"},
			wantErr: true,
		},
		{
			name:    "suffix match at archive root",
			files:   map[string]string{"xmain.lua": "x"},
			wantErr: true,
		},
		{
			name:    "main.lua only below release root",
			files:   map[string]string{"root/nested/main.lua": "x"},
			wantErr: true,
		},
		{
			name:    "dot-slash release root",
			files:   map[string]string{"./release/main.lua": "x"},
			wantErr: false,
		},
		{
			name:    "double dot-slash release root",
			files:   map[string]string{"././release/main.lua": "x"},
			wantErr: false,
		},
		{
			name:    "dot-slash main.lua at archive root",
			files:   map[string]string{"./main.lua": "x"},
			wantErr: true,
		},
		{
			name:    "traversal collapsing to archive root",
			files:   map[string]string{"release/../main.lua": "x"},
			wantErr: true,
		},
		{
			name:    "directory entry masquerading as main.lua",
			files:   map[string]string{"root/main.lua/": ""},
			wantErr: true,
		},
		{
			name: "valid root alongside misleading names",
			files: map[string]string{
				"aaa/xmain.lua": "x",
				"bbb/main.lua":  "x",
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			zipPath := filepath.Join(t.TempDir(), "addon.zip")
			writeTestZip(t, zipPath, tt.files)

			err := ValidateAddonZip(zipPath)
			if tt.wantErr && err == nil {
				t.Fatal("expected validation error, got nil")
			}
			if !tt.wantErr && err != nil {
				t.Fatalf("expected valid archive, got %v", err)
			}
		})
	}
}
