package services

import (
	"ClassicAddonManager/backend/addon"
	"errors"
	"reflect"
	"testing"
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
