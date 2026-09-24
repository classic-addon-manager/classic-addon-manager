package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"errors"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	"github.com/spf13/viper"
)

const updateNotification = "AddonUpdateNotification"

func setupAddonsTxtTest(t *testing.T) string {
	t.Helper()

	dir := t.TempDir()
	prevValue := viper.Get("general.aacpath")
	viper.Set("general.aacpath", dir)
	setInstalledAddonNames(nil)
	t.Cleanup(func() {
		viper.Set("general.aacpath", prevValue)
		setInstalledAddonNames(nil)
	})

	addonDir, err := config.GetAddonDir()
	if err != nil {
		t.Fatalf("GetAddonDir: %v", err)
	}
	return filepath.Join(addonDir, "addons.txt")
}

func assertAddonsTxtLines(t *testing.T, path string, want []string) {
	t.Helper()
	lines, err := file.ReadLines(path)
	if err != nil {
		t.Fatalf("read addons.txt: %v", err)
	}
	if !slices.Equal(lines, want) {
		t.Fatalf("addons.txt = %v, want %v", lines, want)
	}
}

func TestAddonsTxtPreservesUpdateNotification(t *testing.T) {
	path := setupAddonsTxtTest(t)

	seed := []string{"First", updateNotification, "Second"}
	if err := file.WriteLines(path, seed); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}

	names, err := ReadAddonsTxt()
	if err != nil {
		t.Fatalf("ReadAddonsTxt: %v", err)
	}
	if want := []string{"First", "Second"}; !slices.Equal(names, want) {
		t.Fatalf("ReadAddonsTxt = %v, want %v", names, want)
	}
	if got, want := GetInstalledAddonNames(), []string{"First", "Second"}; !slices.Equal(got, want) {
		t.Fatalf("GetInstalledAddonNames = %v, want %v", got, want)
	}

	addons := GetAddons()
	if len(addons) != 2 {
		t.Fatalf("GetAddons returned %d addons, want 2", len(addons))
	}
	for _, a := range addons {
		if a.Name == updateNotification {
			t.Fatalf("GetAddons must not return %q", updateNotification)
		}
	}

	if IsInstalled(updateNotification) {
		t.Fatalf("IsInstalled(%q) = true, want false", updateNotification)
	}
	if !IsInstalled("First") {
		t.Fatal("IsInstalled(First) = false, want true")
	}

	if err := AddToAddonsTxt("Third"); err != nil {
		t.Fatalf("AddToAddonsTxt: %v", err)
	}
	assertAddonsTxtLines(t, path, []string{"First", updateNotification, "Second", "Third"})

	if err := RemoveFromAddonsTxt("First"); err != nil {
		t.Fatalf("RemoveFromAddonsTxt: %v", err)
	}
	assertAddonsTxtLines(t, path, []string{updateNotification, "Second", "Third"})

	if err := AddToAddonsTxt(updateNotification); err != nil {
		t.Fatalf("AddToAddonsTxt(%q): %v", updateNotification, err)
	}
	assertAddonsTxtLines(t, path, []string{updateNotification, "Second", "Third"})
}

func TestSortAddonsTxtPreservesUpdateNotification(t *testing.T) {
	path := setupAddonsTxtTest(t)

	// Second depends on Third, so sorting must move Third ahead of Second.
	localAddonsMu.Lock()
	prev := localAddons
	localAddons = map[string]Addon{
		"Second": {Name: "Second", Dependencies: []string{"Third"}},
	}
	localAddonsMu.Unlock()
	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = prev
		localAddonsMu.Unlock()
	})

	if err := file.WriteLines(path, []string{"Second", updateNotification, "Third"}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}
	if _, err := ReadAddonsTxt(); err != nil {
		t.Fatalf("ReadAddonsTxt: %v", err)
	}

	if err := SortAddonsTxt(); err != nil {
		t.Fatalf("SortAddonsTxt: %v", err)
	}
	assertAddonsTxtLines(t, path, []string{"Third", "Second", updateNotification})

	if got, want := GetInstalledAddonNames(), []string{"Third", "Second"}; !slices.Equal(got, want) {
		t.Fatalf("GetInstalledAddonNames = %v, want %v", got, want)
	}
}

func TestReadAddonsTxtFiltersEveryUpdateNotification(t *testing.T) {
	path := setupAddonsTxtTest(t)

	seed := []string{updateNotification, "First", updateNotification}
	if err := file.WriteLines(path, seed); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}

	names, err := ReadAddonsTxt()
	if err != nil {
		t.Fatalf("ReadAddonsTxt: %v", err)
	}
	if want := []string{"First"}; !slices.Equal(names, want) {
		t.Fatalf("ReadAddonsTxt = %v, want %v", names, want)
	}
	if got, want := GetInstalledAddonNames(), []string{"First"}; !slices.Equal(got, want) {
		t.Fatalf("GetInstalledAddonNames = %v, want %v", got, want)
	}

	if err := AddToAddonsTxt("Second"); err != nil {
		t.Fatalf("AddToAddonsTxt: %v", err)
	}
	assertAddonsTxtLines(t, path, []string{updateNotification, "First", updateNotification, "Second"})
}

func TestReadAddonsTxtSkipsBlankLines(t *testing.T) {
	tests := []struct {
		name    string
		content string
	}{
		{"LF", "First\n\n   \n" + updateNotification + "\n\t\nSecond\n\n"},
		{"CRLF", "First\r\n\r\n  \r\n" + updateNotification + "\r\nSecond\r\n\r\n"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			path := setupAddonsTxtTest(t)

			if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
				t.Fatalf("create addon dir: %v", err)
			}
			if err := os.WriteFile(path, []byte(tt.content), 0644); err != nil {
				t.Fatalf("seed addons.txt: %v", err)
			}

			names, err := ReadAddonsTxt()
			if err != nil {
				t.Fatalf("ReadAddonsTxt: %v", err)
			}
			if want := []string{"First", "Second"}; !slices.Equal(names, want) {
				t.Fatalf("ReadAddonsTxt = %v, want %v", names, want)
			}
			if got, want := GetInstalledAddonNames(), []string{"First", "Second"}; !slices.Equal(got, want) {
				t.Fatalf("GetInstalledAddonNames = %v, want %v", got, want)
			}

			addons := GetAddons()
			if len(addons) != 2 {
				t.Fatalf("GetAddons returned %d addons, want 2", len(addons))
			}
			for _, a := range addons {
				if strings.TrimSpace(a.Name) == "" {
					t.Fatalf("GetAddons returned addon with blank name: %q", a.Name)
				}
			}

			if err := AddToAddonsTxt("Third"); err != nil {
				t.Fatalf("AddToAddonsTxt: %v", err)
			}
			assertAddonsTxtLines(t, path, []string{"First", updateNotification, "Second", "Third"})
		})
	}
}

func TestSortAddonsTxtSkipsBlankLines(t *testing.T) {
	path := setupAddonsTxtTest(t)

	// Second depends on Third, so sorting must move Third ahead of Second.
	localAddonsMu.Lock()
	prev := localAddons
	localAddons = map[string]Addon{
		"Second": {Name: "Second", Dependencies: []string{"Third"}},
	}
	localAddonsMu.Unlock()
	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = prev
		localAddonsMu.Unlock()
	})

	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		t.Fatalf("create addon dir: %v", err)
	}
	if err := os.WriteFile(path, []byte("Second\n\n"+updateNotification+"\n  \nThird\n"), 0644); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}
	if _, err := ReadAddonsTxt(); err != nil {
		t.Fatalf("ReadAddonsTxt: %v", err)
	}

	if err := SortAddonsTxt(); err != nil {
		t.Fatalf("SortAddonsTxt: %v", err)
	}
	assertAddonsTxtLines(t, path, []string{"Third", "Second", updateNotification})

	if got, want := GetInstalledAddonNames(), []string{"Third", "Second"}; !slices.Equal(got, want) {
		t.Fatalf("GetInstalledAddonNames = %v, want %v", got, want)
	}
}

func TestSetupAddonsTxtTestRestoresAACPath(t *testing.T) {
	ambient := viper.Get("general.aacpath")
	t.Cleanup(func() { viper.Set("general.aacpath", ambient) })

	// Establish the unset baseline cleanup must restore: viper treats a nil
	// override as unset.
	viper.Set("general.aacpath", nil)
	if viper.IsSet("general.aacpath") {
		t.Fatal("precondition failed: general.aacpath is set")
	}

	t.Run("sub", func(t *testing.T) {
		setupAddonsTxtTest(t)
		if _, err := config.GetAACDir(); err != nil {
			t.Fatal("setup did not set general.aacpath")
		}
	})

	if viper.IsSet("general.aacpath") {
		t.Fatalf("cleanup left general.aacpath set to %q", viper.GetString("general.aacpath"))
	}
}

func TestGetInstalledAddonNamesReturnsCopy(t *testing.T) {
	path := setupAddonsTxtTest(t)

	seed := []string{"First", updateNotification, "Second"}
	if err := file.WriteLines(path, seed); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}
	if _, err := ReadAddonsTxt(); err != nil {
		t.Fatalf("ReadAddonsTxt: %v", err)
	}

	names := GetInstalledAddonNames()
	if len(names) == 0 {
		t.Fatal("GetInstalledAddonNames returned empty slice")
	}
	names[0] = "Mutated"

	if got := GetInstalledAddonNames(); got[0] != "First" {
		t.Fatalf("mutating returned slice changed cache: %v", got)
	}
	assertAddonsTxtLines(t, path, seed)
}

func assertDirEmpty(t *testing.T, dir string) {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("read dir %s: %v", dir, err)
	}
	if len(entries) != 0 {
		names := make([]string, 0, len(entries))
		for _, e := range entries {
			names = append(names, e.Name())
		}
		t.Fatalf("expected empty dir, found %v", names)
	}
}

func TestCreateAddonsTxtWithoutAACPathWritesNothingToCWD(t *testing.T) {
	cwd := t.TempDir()
	t.Chdir(cwd)

	prev := viper.Get("general.aacpath")
	viper.Set("general.aacpath", nil)
	t.Cleanup(func() { viper.Set("general.aacpath", prev) })

	err := CreateAddonsTxt()
	if !errors.Is(err, config.ErrAACPathNotSet) {
		t.Fatalf("CreateAddonsTxt err = %v, want ErrAACPathNotSet", err)
	}
	assertDirEmpty(t, cwd)
}
