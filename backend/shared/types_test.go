package shared

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestAddonManifestIconDecoding(t *testing.T) {
	t.Run("icon URL is kept when present", func(t *testing.T) {
		body := `[{"name":"FishMe","alias":"FishMe","repo":"author/FishMe","branch":"main","icon":"https://assets.gaijin.dev/icons/76f139d0-a7d5-448c-8afa-7fc98f27cdfb.png"}]`

		var manifests []AddonManifest
		if err := json.NewDecoder(strings.NewReader(body)).Decode(&manifests); err != nil {
			t.Fatalf("decode failed: %v", err)
		}

		if len(manifests) != 1 {
			t.Fatalf("expected 1 manifest, got %d", len(manifests))
		}
		if manifests[0].Icon == nil || *manifests[0].Icon != "https://assets.gaijin.dev/icons/76f139d0-a7d5-448c-8afa-7fc98f27cdfb.png" {
			t.Fatalf("expected icon URL to be kept, got %v", manifests[0].Icon)
		}
	})

	t.Run("icon stays nil when omitted", func(t *testing.T) {
		body := `[{"name":"FishMe","alias":"FishMe","repo":"author/FishMe","branch":"main"}]`

		var manifests []AddonManifest
		if err := json.NewDecoder(strings.NewReader(body)).Decode(&manifests); err != nil {
			t.Fatalf("decode failed: %v", err)
		}

		if manifests[0].Icon != nil {
			t.Fatalf("expected nil icon, got %q", *manifests[0].Icon)
		}
	})
}

func TestAddonManifestDeprecationDecoding(t *testing.T) {
	t.Run("deprecation fields are kept when present", func(t *testing.T) {
		body := `[{"name":"OldMeter","deprecated":true,"deprecation_reason":"Merged into NewMeter","replacement":"NewMeter"}]`

		var manifests []AddonManifest
		if err := json.NewDecoder(strings.NewReader(body)).Decode(&manifests); err != nil {
			t.Fatalf("decode failed: %v", err)
		}

		got := manifests[0]
		if !got.Deprecated {
			t.Fatal("expected deprecated to be true")
		}
		if got.DeprecationReason == nil || *got.DeprecationReason != "Merged into NewMeter" {
			t.Fatalf("expected deprecation reason, got %v", got.DeprecationReason)
		}
		if got.Replacement == nil || *got.Replacement != "NewMeter" {
			t.Fatalf("expected replacement, got %v", got.Replacement)
		}
	})

	t.Run("an addon without the fields is not deprecated", func(t *testing.T) {
		body := `[{"name":"FishMe"}]`

		var manifests []AddonManifest
		if err := json.NewDecoder(strings.NewReader(body)).Decode(&manifests); err != nil {
			t.Fatalf("decode failed: %v", err)
		}

		got := manifests[0]
		if got.Deprecated || got.DeprecationReason != nil || got.Replacement != nil {
			t.Fatalf("expected no deprecation, got %+v", got)
		}
	})
}
