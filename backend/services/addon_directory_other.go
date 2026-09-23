//go:build !windows

package services

import "path/filepath"

func resolveAddonDirectory(path string) (string, error) {
	return filepath.EvalSymlinks(path)
}
