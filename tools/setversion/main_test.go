package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestVersionCommand(t *testing.T) {
	binary := filepath.Join(t.TempDir(), "setversion.exe")
	if output, err := exec.Command("go", "build", "-o", binary, ".").CombinedOutput(); err != nil {
		t.Fatalf("build command: %v\n%s", err, output)
	}

	fixtures := map[string]string{
		"go.mod":                             "module fixture\n",
		"build/config.yml":                   "info:\n  version: \"3.2.0\"\n",
		"build/windows/info.json":            "{\"fixed\": {\"file_version\": \"3.2.0\"}, \"info\": {\"0000\": {\"ProductVersion\": \"3.2.0\"}}}\n",
		"build/windows/wails.exe.manifest":   "<assemblyIdentity name=\"dev.gaijin.classicaddonmanager\" version=\"3.2.0\"/>\n<assemblyIdentity name=\"Microsoft.Windows.Common-Controls\" version=\"6.0.0.0\"/>\n",
		"build/windows/nsis/wails_tools.nsh": "!define INFO_PRODUCTVERSION \"3.2.0\"\n",
		"build/linux/nfpm/nfpm.yaml":         "version: \"3.2.0\"\n",
		"backend/shared/types.go":            "package shared\nvar Version = \"3.2.0\"\n",
	}
	for _, tt := range []struct {
		name    string
		path    string
		old     string
		value   string
		args    []string
		want    string
		wantErr bool
	}{
		{
			name: "set repairs the second JSON field even when the first already matches",
			path: "build/windows/info.json", old: `"ProductVersion": "3.2.0"`, value: `"ProductVersion": "3.1.9"`,
			args: []string{"-set", "3.2.0"}, want: "3.2.0",
		},
		{
			name: "bump rejects drift in the second JSON field",
			path: "build/windows/info.json", old: `"ProductVersion": "3.2.0"`, value: `"ProductVersion": "3.1.9"`,
			args: []string{"-bump", "patch"}, wantErr: true,
		},
		{
			name: "bump rejects a malformed version rather than extracting a valid substring",
			path: "build/config.yml", old: "3.2.0", value: "03.2.0",
			args: []string{"-bump", "patch"}, wantErr: true,
		},
		{
			name: "missing JSON field prevents all writes",
			path: "build/windows/info.json", old: `"ProductVersion": "3.2.0"`, value: "",
			args: []string{"-set", "3.3.0"}, wantErr: true,
		},
		{
			name: "set repairs an invalid manifest without changing the dependency version",
			path: "build/windows/wails.exe.manifest", old: "3.2.0", value: "invalid",
			args: []string{"-set", "v3.3.0"}, want: "3.3.0",
		},
		{
			name: "bump rejects integer overflow without writing",
			old:  "3.2.0", value: "3.2.9223372036854775807",
			args: []string{"-bump", "patch"}, wantErr: true,
		},
	} {
		t.Run(tt.name, func(t *testing.T) {
			dir := t.TempDir()
			initial := make(map[string]string, len(fixtures))
			for path, content := range fixtures {
				if path == tt.path || tt.path == "" {
					content = strings.ReplaceAll(content, tt.old, tt.value)
				}
				initial[path] = content
				fullPath := filepath.Join(dir, path)
				if err := os.MkdirAll(filepath.Dir(fullPath), 0o755); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(fullPath, []byte(content), 0o644); err != nil {
					t.Fatal(err)
				}
			}
			cmd := exec.Command(binary, tt.args...)
			cmd.Dir = dir
			output, err := cmd.CombinedOutput()
			if (err != nil) != tt.wantErr {
				t.Fatalf("command error = %v, want error = %t\n%s", err, tt.wantErr, output)
			}
			for path, content := range fixtures {
				want := strings.ReplaceAll(content, "3.2.0", tt.want)
				if tt.wantErr {
					want = initial[path]
				}
				got, err := os.ReadFile(filepath.Join(dir, path))
				if err != nil {
					t.Fatal(err)
				}
				if string(got) != want {
					t.Errorf("%s: got %q, want %q", path, got, want)
				}
			}
		})
	}
}
