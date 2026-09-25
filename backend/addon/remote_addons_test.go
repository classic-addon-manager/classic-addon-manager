package addon

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"errors"
	"os"
	"path/filepath"
	"sync"
	"sync/atomic"
	"testing"
	"testing/synctest"
	"time"
)

// stubManifestCache resets the manifest cache, installs the given fetch stub
// and a fake clock, and restores everything via t.Cleanup. The returned
// function advances the fake clock.
func stubManifestCache(t *testing.T, fetch func() ([]shared.AddonManifest, error)) func(time.Duration) {
	t.Helper()
	// The logger singleton opens app.log on first use and never closes it, so
	// point the config dir at the shared temp dir rather than t.TempDir()
	// (Windows cannot remove a temp dir holding an open file).
	t.Setenv("APPDATA", os.TempDir())
	t.Setenv("XDG_CONFIG_HOME", os.TempDir())

	manifestCacheMu.Lock()
	manifestCache = nil
	manifestCacheUntil = time.Time{}
	manifestInflight = nil
	manifestGen = 0
	manifestCacheMu.Unlock()

	origFetch := fetchAddonManifest
	origNow := now

	var clockMu sync.Mutex
	fakeNow := time.Now()
	fetchAddonManifest = fetch
	now = func() time.Time {
		clockMu.Lock()
		defer clockMu.Unlock()
		return fakeNow
	}

	t.Cleanup(func() {
		fetchAddonManifest = origFetch
		now = origNow
		manifestCacheMu.Lock()
		manifestCache = nil
		manifestCacheUntil = time.Time{}
		manifestInflight = nil
		manifestGen = 0
		manifestCacheMu.Unlock()
	})

	return func(d time.Duration) {
		clockMu.Lock()
		fakeNow = fakeNow.Add(d)
		clockMu.Unlock()
	}
}

func TestGetAddonManifestCachesWithinTTL(t *testing.T) {
	var calls int
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return []shared.AddonManifest{{Name: "cached"}}, nil
	})

	first, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("first call returned error: %v", err)
	}
	second, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("second call returned error: %v", err)
	}

	if calls != 1 {
		t.Fatalf("fetch called %d times, want 1", calls)
	}
	if len(first) != 1 || first[0].Name != "cached" {
		t.Fatalf("first call returned %+v, want [cached]", first)
	}
	if len(second) != 1 || second[0].Name != "cached" {
		t.Fatalf("second call returned %+v, want [cached]", second)
	}
}

func TestGetAddonManifestRefetchesAfterTTL(t *testing.T) {
	var calls int
	advance := stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return []shared.AddonManifest{{Name: "fetch-" + string(rune('0'+calls))}}, nil
	})

	first, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("first call returned error: %v", err)
	}
	advance(addonManifestTTL + time.Second)
	second, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("second call returned error: %v", err)
	}

	if calls != 2 {
		t.Fatalf("fetch called %d times, want 2", calls)
	}
	if first[0].Name == second[0].Name {
		t.Fatalf("expected new data after TTL, got %q twice", second[0].Name)
	}
}

func TestGetAddonManifestServesCachedOnError(t *testing.T) {
	var calls int
	fail := false
	name := "v1"
	advance := stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		if fail {
			return nil, errors.New("fetch failed")
		}
		return []shared.AddonManifest{{Name: name}}, nil
	})

	got, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("initial fetch returned error: %v", err)
	}
	if len(got) != 1 || got[0].Name != "v1" {
		t.Fatalf("initial fetch returned %+v, want [v1]", got)
	}

	advance(addonManifestTTL + time.Second)
	fail = true
	got, err = GetAddonManifest()
	if err != nil {
		t.Fatalf("expired fetch error returned error, want stale fallback: %v", err)
	}
	if len(got) != 1 || got[0].Name != "v1" {
		t.Fatalf("expired fetch error returned %+v, want last cached [v1]", got)
	}
	if calls != 2 {
		t.Fatalf("fetch called %d times, want 2", calls)
	}

	// Errors are not cached: the next call fetches again.
	got, err = GetAddonManifest()
	if err != nil {
		t.Fatalf("second error returned error, want stale fallback: %v", err)
	}
	if len(got) != 1 || got[0].Name != "v1" {
		t.Fatalf("second error returned %+v, want last cached [v1]", got)
	}
	if calls != 3 {
		t.Fatalf("fetch called %d times, want 3", calls)
	}

	// A later success replaces the catalog.
	fail = false
	name = "v2"
	got, err = GetAddonManifest()
	if err != nil {
		t.Fatalf("recovery call returned error: %v", err)
	}
	if len(got) != 1 || got[0].Name != "v2" || calls != 4 {
		t.Fatalf("recovery call returned %+v after %d fetches, want [v2] after 4", got, calls)
	}
}

func TestGetAddonManifestErrorWithoutCache(t *testing.T) {
	var calls int
	fetchErr := errors.New("fetch failed")
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return nil, fetchErr
	})

	got, err := GetAddonManifest()
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !errors.Is(err, fetchErr) {
		t.Fatalf("expected error wrapping %q, got %v", fetchErr, err)
	}
	if got != nil {
		t.Fatalf("expected nil slice, got %+v", got)
	}

	GetAddonManifest()
	if calls != 2 {
		t.Fatalf("fetch called %d times, want 2 (errors are not cached)", calls)
	}
}

func TestGetAddonManifestEmptyCatalogIsValid(t *testing.T) {
	var calls int
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return nil, nil
	})

	got, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("empty catalog returned error: %v", err)
	}
	if got == nil {
		t.Fatal("expected non-nil empty slice, got nil")
	}
	if len(got) != 0 {
		t.Fatalf("expected empty slice, got %+v", got)
	}

	if _, err := GetAddonManifest(); err != nil {
		t.Fatalf("cached empty catalog returned error: %v", err)
	}
	if calls != 1 {
		t.Fatalf("fetch called %d times, want 1 (empty catalog is cached)", calls)
	}
}

func TestInvalidateAddonManifestCache(t *testing.T) {
	var calls int
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return []shared.AddonManifest{{Name: "data"}}, nil
	})

	if _, err := GetAddonManifest(); err != nil {
		t.Fatalf("initial call returned error: %v", err)
	}
	InvalidateAddonManifestCache()
	if _, err := GetAddonManifest(); err != nil {
		t.Fatalf("post-invalidation call returned error: %v", err)
	}

	if calls != 2 {
		t.Fatalf("fetch called %d times, want 2 after invalidation", calls)
	}
}

func TestGetAddonManifestConcurrentFetch(t *testing.T) {
	var calls atomic.Int32
	var release chan struct{}

	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls.Add(1)
		<-release
		return []shared.AddonManifest{{Name: "concurrent"}}, nil
	})
	// Initialize the logger singleton outside the bubble so synctest does not
	// track goroutines or files it may create.
	logger.Info("starting manifest concurrency test")

	synctest.Test(t, func(t *testing.T) {
		release = make(chan struct{})

		const goroutines = 10
		results := make([][]shared.AddonManifest, goroutines)
		errs := make([]error, goroutines)
		var wg sync.WaitGroup
		for i := 0; i < goroutines; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				results[i], errs[i] = GetAddonManifest()
			}(i)
		}

		synctest.Wait()
		close(release)
		wg.Wait()

		for i, res := range results {
			if errs[i] != nil {
				t.Fatalf("goroutine %d got error: %v", i, errs[i])
			}
			if len(res) != 1 || res[0].Name != "concurrent" {
				t.Fatalf("goroutine %d got %+v, want [concurrent]", i, res)
			}
		}
	})

	if got := calls.Load(); got != 1 {
		t.Fatalf("fetch called %d times, want 1", got)
	}
}

func TestGetAddonManifestConcurrentFailureSharesFetch(t *testing.T) {
	var fail atomic.Bool
	var calls atomic.Int32
	var release chan struct{}

	advance := stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls.Add(1)
		if fail.Load() {
			<-release
			return nil, errors.New("fetch failed")
		}
		return []shared.AddonManifest{{Name: "seeded"}}, nil
	})

	got, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("seed fetch returned error: %v", err)
	}
	if len(got) != 1 || got[0].Name != "seeded" {
		t.Fatalf("seed fetch returned %+v, want [seeded]", got)
	}
	advance(addonManifestTTL + time.Second)
	logger.Info("starting manifest concurrency test")

	synctest.Test(t, func(t *testing.T) {
		release = make(chan struct{})
		fail.Store(true)

		const goroutines = 10
		results := make([][]shared.AddonManifest, goroutines)
		errs := make([]error, goroutines)
		var wg sync.WaitGroup
		for i := 0; i < goroutines; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				results[i], errs[i] = GetAddonManifest()
			}(i)
		}

		synctest.Wait()
		close(release)
		wg.Wait()

		for i, res := range results {
			if errs[i] != nil {
				t.Fatalf("goroutine %d got error, want stale fallback: %v", i, errs[i])
			}
			if len(res) != 1 || res[0].Name != "seeded" {
				t.Fatalf("goroutine %d got %+v, want last cached [seeded]", i, res)
			}
		}
	})

	if got := calls.Load(); got != 2 {
		t.Fatalf("fetch called %d times, want 2 (1 seed + 1 shared failure)", got)
	}
}

func TestInvalidateAddonManifestCacheDuringFetch(t *testing.T) {
	var calls atomic.Int32
	var release chan struct{}

	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		call := calls.Add(1)
		if call == 1 {
			<-release
			return []shared.AddonManifest{{Name: "v1"}}, nil
		}
		return []shared.AddonManifest{{Name: "v2"}}, nil
	})
	logger.Info("starting manifest invalidation test")

	synctest.Test(t, func(t *testing.T) {
		release = make(chan struct{})

		var aResult []shared.AddonManifest
		var aErr error
		var wg sync.WaitGroup
		wg.Add(1)
		go func() {
			defer wg.Done()
			aResult, aErr = GetAddonManifest()
		}()

		// A is now blocked inside fetch #1.
		synctest.Wait()

		InvalidateAddonManifestCache()

		// Must not join fetch #1: this call starts fetch #2.
		got, err := GetAddonManifest()
		if err != nil {
			t.Fatalf("post-invalidation call returned error: %v", err)
		}
		if len(got) != 1 || got[0].Name != "v2" {
			t.Fatalf("post-invalidation call returned %+v, want [v2]", got)
		}

		close(release)
		wg.Wait()
		if aErr != nil {
			t.Fatalf("in-flight caller got error: %v", aErr)
		}
		if len(aResult) != 1 || aResult[0].Name != "v1" {
			t.Fatalf("in-flight caller got %+v, want [v1]", aResult)
		}

		// Still within the TTL: proves the superseded fetch did not overwrite
		// the cache or its expiry.
		got, err = GetAddonManifest()
		if err != nil {
			t.Fatalf("cached call returned error: %v", err)
		}
		if len(got) != 1 || got[0].Name != "v2" {
			t.Fatalf("cached call returned %+v, want [v2]", got)
		}
	})

	if got := calls.Load(); got != 2 {
		t.Fatalf("fetch called %d times, want 2", got)
	}
}

func TestGetAddonManifestReturnedSliceIsCopy(t *testing.T) {
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		return []shared.AddonManifest{{Name: "original"}}, nil
	})

	first, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("first call returned error: %v", err)
	}
	first[0].Name = "mutated"

	second, err := GetAddonManifest()
	if err != nil {
		t.Fatalf("second call returned error: %v", err)
	}
	if second[0].Name != "original" {
		t.Fatalf("mutating returned slice changed cache: got %q", second[0].Name)
	}
}

// setupRemoteAddonTest prepares the AAC dir, cache dir, config dir, and
// managed-addon state that InstallAddon/UpdateAddon touch.
func setupRemoteAddonTest(t *testing.T) (addonDir string) {
	t.Helper()

	// The logger singleton opens app.log on first use and never closes it, so
	// initialize it against the shared temp dir before pointing the config dir
	// at t.TempDir() (Windows cannot remove a temp dir holding an open file).
	t.Setenv("APPDATA", os.TempDir())
	t.Setenv("XDG_CONFIG_HOME", os.TempDir())
	logger.Info("starting remote addon install test")
	dataDir := t.TempDir()
	t.Setenv("APPDATA", dataDir)
	t.Setenv("XDG_CONFIG_HOME", dataDir)

	localAddonsMu.Lock()
	localAddons = make(map[string]Addon)
	localAddonsMu.Unlock()
	t.Cleanup(func() {
		localAddonsMu.Lock()
		localAddons = nil
		localAddonsMu.Unlock()
	})

	return setupInstallZipTest(t)
}

// stubRemoteAddons replaces the release fetch and download/extract steps and
// restores them via t.Cleanup.
func stubRemoteAddons(t *testing.T, getRelease func(name, version string) (api.Release, error), download func(manifest shared.AddonManifest, version string) error) {
	t.Helper()

	origGet := getAddonRelease
	origDownload := downloadAndExtract
	getAddonRelease = getRelease
	downloadAndExtract = download
	t.Cleanup(func() {
		getAddonRelease = origGet
		downloadAndExtract = origDownload
	})
}

// fakeExtractedRelease simulates a downloaded release by writing the extracted
// layout MoveAddonRelease expects: <cache>/<name>/release-root/main.lua.
func fakeExtractedRelease(mainLua string) func(manifest shared.AddonManifest, version string) error {
	return func(manifest shared.AddonManifest, _ string) error {
		cacheDir, err := config.GetCacheDir()
		if err != nil {
			return err
		}
		root := filepath.Join(cacheDir, manifest.Name, "release-root")
		if err := os.MkdirAll(root, 0755); err != nil {
			return err
		}
		return os.WriteFile(filepath.Join(root, "main.lua"), []byte(mainLua), 0644)
	}
}

func TestInstallAddonReleaseMetadataFailureLeavesNothingInstalled(t *testing.T) {
	addonDir := setupRemoteAddonTest(t)
	manifest := shared.AddonManifest{Name: "MyAddon", Repo: "example/repo"}

	fetchErr := errors.New("release fetch failed")
	stubRemoteAddons(t,
		func(_, _ string) (api.Release, error) {
			return api.Release{}, fetchErr
		},
		fakeExtractedRelease("new main"),
	)

	ok, err := InstallAddon(manifest, "")
	if err == nil || ok {
		t.Fatalf("InstallAddon = (%v, %v), want (false, error)", ok, err)
	}
	if !errors.Is(err, fetchErr) {
		t.Fatalf("InstallAddon error = %v, want %v", err, fetchErr)
	}

	if file.FileExists(filepath.Join(addonDir, manifest.Name)) {
		t.Fatal("addon directory must not exist after a failed install")
	}
	if IsInstalled(manifest.Name) {
		t.Fatal("IsInstalled = true, want false")
	}
	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{})
	if addon := FindLocalAddonByName(manifest.Name); addon != nil {
		t.Fatalf("FindLocalAddonByName = %+v, want nil", addon)
	}
}

func TestUpdateAddonReleaseMetadataFailureKeepsExistingInstall(t *testing.T) {
	addonDir := setupRemoteAddonTest(t)
	manifest := shared.AddonManifest{Name: "MyAddon", Repo: "example/repo"}
	dest := filepath.Join(addonDir, manifest.Name)

	if err := os.MkdirAll(dest, 0755); err != nil {
		t.Fatalf("seed addon dir: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dest, "main.lua"), []byte("old"), 0644); err != nil {
		t.Fatalf("seed main.lua: %v", err)
	}
	if err := file.WriteLines(filepath.Join(addonDir, "addons.txt"), []string{manifest.Name}); err != nil {
		t.Fatalf("seed addons.txt: %v", err)
	}
	if _, err := ReadAddonsTxt(); err != nil {
		t.Fatalf("load addons.txt: %v", err)
	}
	AddManagedAddon(manifest, api.Release{TagName: "v1"})

	fetchErr := errors.New("release fetch failed")
	stubRemoteAddons(t,
		func(_, _ string) (api.Release, error) {
			return api.Release{}, fetchErr
		},
		fakeExtractedRelease("new"),
	)

	ok, err := UpdateAddon(manifest, "v2")
	if err == nil || ok {
		t.Fatalf("UpdateAddon = (%v, %v), want (false, error)", ok, err)
	}

	data, err := os.ReadFile(filepath.Join(dest, "main.lua"))
	if err != nil {
		t.Fatalf("read main.lua: %v", err)
	}
	if string(data) != "old" {
		t.Fatalf("main.lua = %q, want untouched original %q", string(data), "old")
	}
	addon := FindLocalAddonByName(manifest.Name)
	if addon == nil {
		t.Fatal("managed addon entry must be kept")
	}
	if addon.Version != "v1" {
		t.Fatalf("managed addon version = %q, want %q", addon.Version, "v1")
	}
}

func TestInstallAddonRecordsManagedRelease(t *testing.T) {
	addonDir := setupRemoteAddonTest(t)
	manifest := shared.AddonManifest{Name: "MyAddon", Repo: "example/repo"}

	stubRemoteAddons(t,
		func(_, _ string) (api.Release, error) {
			return api.Release{TagName: "v2.0.0"}, nil
		},
		fakeExtractedRelease("new main"),
	)

	ok, err := InstallAddon(manifest, "v2.0.0")
	if err != nil || !ok {
		t.Fatalf("InstallAddon = (%v, %v), want (true, nil)", ok, err)
	}

	data, err := os.ReadFile(filepath.Join(addonDir, manifest.Name, "main.lua"))
	if err != nil {
		t.Fatalf("read main.lua: %v", err)
	}
	if string(data) != "new main" {
		t.Fatalf("main.lua = %q, want %q", string(data), "new main")
	}
	assertAddonsTxtLines(t, filepath.Join(addonDir, "addons.txt"), []string{manifest.Name})

	addon := FindLocalAddonByName(manifest.Name)
	if addon == nil {
		t.Fatal("managed addon entry must be recorded")
	}
	if !addon.IsManaged {
		t.Fatal("managed addon entry must have IsManaged = true")
	}
	if addon.Version != "v2.0.0" {
		t.Fatalf("managed addon version = %q, want %q", addon.Version, "v2.0.0")
	}
}
