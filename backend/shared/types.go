package shared

import (
	"time"
)

var Version string
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
