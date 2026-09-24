package main

import "testing"

func TestIsAuthDeeplink(t *testing.T) {
	tests := []struct {
		name string
		raw  string
		want bool
	}{
		{"auth with token", "classicaddonmanager://auth?t=abc", true},
		{"scheme case-insensitive", "ClassicAddonManager://auth?t=abc", true},
		{"auth with trailing slash", "classicaddonmanager://auth/?t=abc", true},
		{"auth without token", "classicaddonmanager://auth", true},
		{"empty", "", false},
		{"flag double dash", "--check-updates", false},
		{"flag single dash", "-check-updates", false},
		{"windows path", `C:\foo\bar.txt`, false},
		{"unix path", "/tmp/file", false},
		{"other scheme", "https://auth?t=abc", false},
		{"other host", "classicaddonmanager://other?t=abc", false},
		{"opaque url", "classicaddonmanager:auth?t=abc", false},
		{"newline injection", "classicaddonmanager://auth?t=abc\n--check-updates", false},
		{"userinfo host spoof", "classicaddonmanager://user@evil/auth", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := isAuthDeeplink(tt.raw); got != tt.want {
				t.Errorf("isAuthDeeplink(%q) = %v, want %v", tt.raw, got, tt.want)
			}
		})
	}
}
