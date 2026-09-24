package addon

import (
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"errors"
	"os"
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

	first := GetAddonManifest()
	second := GetAddonManifest()

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

	first := GetAddonManifest()
	advance(addonManifestTTL + time.Second)
	second := GetAddonManifest()

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

	if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "v1" {
		t.Fatalf("initial fetch returned %+v, want [v1]", got)
	}

	advance(addonManifestTTL + time.Second)
	fail = true
	if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "v1" {
		t.Fatalf("expired fetch error returned %+v, want last cached [v1]", got)
	}
	if calls != 2 {
		t.Fatalf("fetch called %d times, want 2", calls)
	}

	// Errors are not cached: the next call fetches again.
	if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "v1" {
		t.Fatalf("second error returned %+v, want last cached [v1]", got)
	}
	if calls != 3 {
		t.Fatalf("fetch called %d times, want 3", calls)
	}

	// A later success replaces the catalog.
	fail = false
	name = "v2"
	if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "v2" || calls != 4 {
		t.Fatalf("recovery call returned %+v after %d fetches, want [v2] after 4", got, calls)
	}
}

func TestGetAddonManifestEmptyOnErrorWithoutCache(t *testing.T) {
	var calls int
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return nil, errors.New("fetch failed")
	})

	got := GetAddonManifest()
	if got == nil {
		t.Fatal("expected non-nil empty slice, got nil")
	}
	if len(got) != 0 {
		t.Fatalf("expected empty slice, got %+v", got)
	}

	GetAddonManifest()
	if calls != 2 {
		t.Fatalf("fetch called %d times, want 2 (errors are not cached)", calls)
	}
}

func TestInvalidateAddonManifestCache(t *testing.T) {
	var calls int
	stubManifestCache(t, func() ([]shared.AddonManifest, error) {
		calls++
		return []shared.AddonManifest{{Name: "data"}}, nil
	})

	GetAddonManifest()
	InvalidateAddonManifestCache()
	GetAddonManifest()

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
		var wg sync.WaitGroup
		for i := 0; i < goroutines; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				results[i] = GetAddonManifest()
			}(i)
		}

		synctest.Wait()
		close(release)
		wg.Wait()

		for i, res := range results {
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

	if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "seeded" {
		t.Fatalf("seed fetch returned %+v, want [seeded]", got)
	}
	advance(addonManifestTTL + time.Second)
	logger.Info("starting manifest concurrency test")

	synctest.Test(t, func(t *testing.T) {
		release = make(chan struct{})
		fail.Store(true)

		const goroutines = 10
		results := make([][]shared.AddonManifest, goroutines)
		var wg sync.WaitGroup
		for i := 0; i < goroutines; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				results[i] = GetAddonManifest()
			}(i)
		}

		synctest.Wait()
		close(release)
		wg.Wait()

		for i, res := range results {
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
		var wg sync.WaitGroup
		wg.Add(1)
		go func() {
			defer wg.Done()
			aResult = GetAddonManifest()
		}()

		// A is now blocked inside fetch #1.
		synctest.Wait()

		InvalidateAddonManifestCache()

		// Must not join fetch #1: this call starts fetch #2.
		if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "v2" {
			t.Fatalf("post-invalidation call returned %+v, want [v2]", got)
		}

		close(release)
		wg.Wait()
		if len(aResult) != 1 || aResult[0].Name != "v1" {
			t.Fatalf("in-flight caller got %+v, want [v1]", aResult)
		}

		// Still within the TTL: proves the superseded fetch did not overwrite
		// the cache or its expiry.
		if got := GetAddonManifest(); len(got) != 1 || got[0].Name != "v2" {
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

	first := GetAddonManifest()
	first[0].Name = "mutated"

	second := GetAddonManifest()
	if second[0].Name != "original" {
		t.Fatalf("mutating returned slice changed cache: got %q", second[0].Name)
	}
}
