package shared

import (
	"time"
)

var Version = "3.0.6"
var AuthToken string

type AddonManifest struct {
	Name         string    `json:"name"`
	Alias        string    `json:"alias"`
	Dependencies []string  `json:"dependencies"`
	Description  string    `json:"description"`
	Author       string    `json:"author"`
	Repo         string    `json:"repo"`
	Branch       string    `json:"branch"`
	Tags         []string  `json:"tags"`
	Downloads    int       `json:"downloads"`
	LikePercent  *int8     `json:"like_percentage"`
	Kofi         *string   `json:"kofi,omitempty"`
	AddedAt      time.Time `json:"added_at"`
	Warning      *string   `json:"warning,omitempty"`
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
