package services

import (
	"ClassicAddonManager/backend/addon"
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"runtime"
	"testing"

	"github.com/spf13/viper"
)

func stubUninstallDeps(t *testing.T) {
	t.Helper()
	origIsInstalled := isAddonInstalled
	origFind := findLocalAddonByName
	origRemoveTxt := removeFromAddonsTxt
	origRemoveDir := removeAddonDirectory
	origRemoveManaged := removeManagedAddon
	origSort := sortAddonsTxt
	origUnsub := unsubscribeFromAddon
	t.Cleanup(func() {
		isAddonInstalled = origIsInstalled
		findLocalAddonByName = origFind
		removeFromAddonsTxt = origRemoveTxt
		removeAddonDirectory = origRemoveDir
		removeManagedAddon = origRemoveManaged
		sortAddonsTxt = origSort
		unsubscribeFromAddon = origUnsub
	})
}

func recordUninstallOrder(order *[]string) {
	isAddonInstalled = func(string) bool { return true }
	findLocalAddonByName = func(name string) *addon.Addon {
		return &addon.Addon{Name: name}
	}
	removeFromAddonsTxt = func(string) error {
		*order = append(*order, "addons.txt")
		return nil
	}
	removeAddonDirectory = func(string) (bool, error) {
		*order = append(*order, "directory")
		return true, nil
	}
	removeManagedAddon = func(string) bool {
		*order = append(*order, "managed")
		return true
	}
	sortAddonsTxt = func() error {
		*order = append(*order, "sort")
		return nil
	}
	unsubscribeFromAddon = func(string) {
		*order = append(*order, "unsubscribe")
	}
}

func TestLocalAddonService_UninstallAddon_ManagedOrder(t *testing.T) {
	stubUninstallDeps(t)
	var order []string
	recordUninstallOrder(&order)

	s := &LocalAddonService{}
	if !s.UninstallAddon("managed-addon") {
		t.Fatalf("expected uninstall to succeed")
	}

	want := []string{"addons.txt", "directory", "managed", "sort", "unsubscribe"}
	if !reflect.DeepEqual(order, want) {
		t.Fatalf("expected order %v, got %v", want, order)
	}
}

func TestLocalAddonService_UninstallAddon_AddonsTxtError(t *testing.T) {
	stubUninstallDeps(t)
	var order []string
	recordUninstallOrder(&order)
	removeFromAddonsTxt = func(string) error {
		order = append(order, "addons.txt")
		return errors.New("write failed")
	}

	s := &LocalAddonService{}
	if s.UninstallAddon("managed-addon") {
		t.Fatalf("expected uninstall to fail on addons.txt error")
	}

	want := []string{"addons.txt"}
	if !reflect.DeepEqual(order, want) {
		t.Fatalf("expected order %v, got %v", want, order)
	}
}

func TestLocalAddonService_UninstallAddon_DirectoryFailure(t *testing.T) {
	stubUninstallDeps(t)
	var order []string
	recordUninstallOrder(&order)
	removeAddonDirectory = func(string) (bool, error) {
		order = append(order, "directory")
		return false, errors.New("remove failed")
	}

	s := &LocalAddonService{}
	if s.UninstallAddon("managed-addon") {
		t.Fatalf("expected uninstall to fail on directory removal error")
	}

	want := []string{"addons.txt", "directory"}
	if !reflect.DeepEqual(order, want) {
		t.Fatalf("expected order %v, got %v", want, order)
	}
}

func TestLocalAddonService_UninstallAddon_ManualAddon(t *testing.T) {
	stubUninstallDeps(t)
	var order []string
	recordUninstallOrder(&order)
	findLocalAddonByName = func(string) *addon.Addon { return nil }

	s := &LocalAddonService{}
	if !s.UninstallAddon("manual-addon") {
		t.Fatalf("expected uninstall to succeed")
	}

	want := []string{"addons.txt", "directory", "sort"}
	if !reflect.DeepEqual(order, want) {
		t.Fatalf("expected order %v, got %v", want, order)
	}
}

func TestLocalAddonService_OpenDirectory_StaysInsideAddonRoot(t *testing.T) {
	aacDir := t.TempDir()
	root := filepath.Join(aacDir, "Addon")
	valid := filepath.Join(root, "Valid Addon")
	outside := filepath.Join(aacDir, "Outside")
	for _, dir := range []string{valid, outside} {
		if err := os.MkdirAll(dir, 0700); err != nil {
			t.Fatal(err)
		}
	}

	previous := viper.Get("general.aacpath")
	viper.Set("general.aacpath", aacDir)
	originalOpen := openAddonDirectory
	t.Cleanup(func() {
		viper.Set("general.aacpath", previous)
		openAddonDirectory = originalOpen
	})

	service := &LocalAddonService{}
	for _, tc := range []struct {
		name    string
		allowed bool
	}{
		{name: "Valid Addon", allowed: true},
		{name: "../Outside"},
		{name: `..\Outside`},
		{name: outside},
		{name: "Valid Addon/../Outside"},
		{name: "."},
		{name: ""},
	} {
		t.Run(tc.name, func(t *testing.T) {
			var opened string
			openAddonDirectory = func(path string) error {
				opened = path
				return nil
			}
			err := service.OpenDirectory(tc.name)
			if tc.allowed {
				if err != nil || opened != valid {
					t.Fatalf("OpenDirectory(%q) = (%q, %v), want (%q, nil)", tc.name, opened, err, valid)
				}
			} else if err == nil || opened != "" {
				t.Fatalf("OpenDirectory(%q) = (%q, %v), want rejection without opening", tc.name, opened, err)
			}
		})
	}

	t.Run("escaping link", func(t *testing.T) {
		link := filepath.Join(root, "Escaping Link")
		if err := os.Symlink(outside, link); err != nil {
			if runtime.GOOS != "windows" {
				t.Skipf("symlink unavailable: %v", err)
			}
			// Junctions need no symlink privilege, both links must be rejected.
			if output, junctionErr := exec.Command("cmd", "/c", "mklink", "/J", link, outside).CombinedOutput(); junctionErr != nil {
				t.Skipf("symlink and junction unavailable: %v; %s", err, output)
			}
		}
		var opened string
		openAddonDirectory = func(path string) error {
			opened = path
			return nil
		}
		if err := service.OpenDirectory("Escaping Link"); err == nil || opened != "" {
			t.Fatalf("escaping link = (%q, %v), want rejection without opening", opened, err)
		}
	})

	t.Run("internal link", func(t *testing.T) {
		link := filepath.Join(root, "Internal Link")
		if err := os.Symlink(valid, link); err != nil {
			if runtime.GOOS != "windows" {
				t.Skipf("symlink unavailable: %v", err)
			}
			if output, junctionErr := exec.Command("cmd", "/c", "mklink", "/J", link, valid).CombinedOutput(); junctionErr != nil {
				t.Skipf("symlink and junction unavailable: %v; %s", err, output)
			}
		}
		var opened string
		openAddonDirectory = func(path string) error {
			opened = path
			return nil
		}
		if err := service.OpenDirectory("Internal Link"); err != nil || opened != valid {
			t.Fatalf("internal link = (%q, %v), want (%q, nil)", opened, err, valid)
		}
	})

	if runtime.GOOS == "windows" {
		t.Run("nested junction", func(t *testing.T) {
			child := filepath.Join(outside, "Child")
			if err := os.Mkdir(child, 0700); err != nil {
				t.Fatal(err)
			}
			intermediate := filepath.Join(root, "Intermediate")
			link := filepath.Join(root, "Chained Link")
			for _, pair := range [][2]string{{intermediate, outside}, {link, filepath.Join(intermediate, "Child")}} {
				if output, err := exec.Command("cmd", "/c", "mklink", "/J", pair[0], pair[1]).CombinedOutput(); err != nil {
					t.Skipf("junction unavailable: %v; %s", err, output)
				}
			}
			var opened string
			openAddonDirectory = func(path string) error {
				opened = path
				return nil
			}
			if err := service.OpenDirectory("Chained Link"); err == nil || opened != "" {
				t.Fatalf("nested junction = (%q, %v), want rejection without opening", opened, err)
			}
		})
	}
}
