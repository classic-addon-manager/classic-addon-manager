package shared

import (
	"time"
)

var Version = "3.2.1"

type AuthSessionState struct {
	Token string `json:"token"`
}

type UIPreferences struct {
	AccentColor   string `json:"accentColor"`
	AddonViewMode string `json:"addonViewMode"`
}

type AddonManifest struct {
	Name         string    `json:"name"`
	Alias        string    `json:"alias"`
	Dependencies []string  `json:"dependencies"`
	Description  string    `json:"description"`
	Author       string    `json:"author"`
	Repo         string    `json:"repo"`
	Branch       string    `json:"branch"`
	Icon         *string   `json:"icon,omitempty"`
	Tags         []string  `json:"tags"`
	Downloads    int       `json:"downloads"`
	LikePercent  *int8     `json:"like_percentage"`
	Kofi         *string   `json:"kofi,omitempty"`
	AddedAt      time.Time `json:"added_at"`
	Warning      *string   `json:"warning,omitempty"`
	Library      bool      `json:"library"`

	// A deprecated addon stays in the catalog so installed copies can be flagged for removal.
	Deprecated        bool    `json:"deprecated"`
	DeprecationReason *string `json:"deprecation_reason,omitempty"`
	// Name of the catalog addon that replaces this one, when the developer named one.
	Replacement *string `json:"replacement,omitempty"`
}

type DependencyInfo struct {
	Manifest    AddonManifest `json:"manifest"`
	IsInstalled bool          `json:"isInstalled"`
	Depth       int           `json:"depth"`
}

type DependencyResolutionResult struct {
	Dependencies []DependencyInfo `json:"dependencies"`
	Errors       []string         `json:"errors"`
}

type AddonInstallStatus struct {
	Name    string `json:"name"`
	Alias   string `json:"alias"`
	Success bool   `json:"success"`
	Skipped bool   `json:"skipped"`
	Error   string `json:"error,omitempty"`
}

type InstallWithDependenciesResult struct {
	Success            bool                 `json:"success"`
	DependencyWarnings []string             `json:"dependencyWarnings"`
	Dependencies       []AddonInstallStatus `json:"dependencies"`
	MainAddon          AddonInstallStatus   `json:"mainAddon"`
}
