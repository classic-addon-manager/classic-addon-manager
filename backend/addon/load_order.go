package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"

	"fmt"
	"path/filepath"
	"slices"
	"strings"
)

// node colors for cycle detection during the depth-first topological sort.
const (
	nodeWhite = iota // unvisited
	nodeGray         // on the current DFS stack
	nodeBlack        // fully processed
)

// resolveLoadOrder returns names ordered so that every addon appears after all
// the addons it (transitively) depends on. Only dependency edges whose target
// is also present in names are followed, so the result is a reordering of the
// provided names and never adds or removes entries.
// A circular dependency yields an error and no ordering.
func resolveLoadOrder(names []string, manifestByName map[string]shared.AddonManifest) ([]string, error) {
	present := make(map[string]struct{}, len(names))
	for _, name := range names {
		present[name] = struct{}{}
	}

	color := make(map[string]int, len(names))
	ordered := make([]string, 0, len(names))

	var visit func(name string, path []string) error
	visit = func(name string, path []string) error {
		switch color[name] {
		case nodeBlack:
			return nil
		case nodeGray:
			cycle := append(append([]string{}, path...), name)
			return fmt.Errorf("circular dependency detected: %s", strings.Join(cycle, " -> "))
		}

		color[name] = nodeGray

		if manifest, ok := manifestByName[name]; ok {
			for _, dep := range manifest.Dependencies {
				if _, installed := present[dep]; !installed {
					continue
				}
				if err := visit(dep, append(path, name)); err != nil {
					return err
				}
			}
		}

		color[name] = nodeBlack
		ordered = append(ordered, name)
		return nil
	}

	for _, name := range names {
		if err := visit(name, nil); err != nil {
			return nil, err
		}
	}

	return ordered, nil
}

// SortAddonsTxt rewrites addons.txt so that dependencies are loaded before the
// addons that depend on them, fully resolving transitive dependencies. It only
// reorders addons that are already listed; it does not add missing
// dependencies. On a circular dependency the file is left untouched, the error
// is logged, and it is returned to the caller. Call this after any mutation of
// addons.txt (adding or removing entries).
func SortAddonsTxt() error {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()

	if len(installedAddonNames) == 0 {
		return nil
	}

	manifests := GetAddonManifest()
	if len(manifests) == 0 {
		// Without manifests we cannot determine dependency order. Leave the
		// existing order untouched rather than breaking offline usage.
		logger.Warn("SortAddonsTxt: no addon manifests available, skipping load-order sort")
		return nil
	}

	manifestByName := make(map[string]shared.AddonManifest, len(manifests))
	for _, manifest := range manifests {
		manifestByName[manifest.Name] = manifest
	}

	ordered, err := resolveLoadOrder(installedAddonNames, manifestByName)
	if err != nil {
		logger.Error("SortAddonsTxt: failed to resolve load order:", err)
		return err
	}

	if slices.Equal(ordered, installedAddonNames) {
		return nil
	}

	if writeErr := file.WriteLines(filepath.Join(config.GetAddonDir(), "addons.txt"), ordered); writeErr != nil {
		logger.Error("SortAddonsTxt: failed to write addons.txt:", writeErr)
		// Rollback the in-memory cache from disk if the write failed.
		lines, readErr := file.ReadLines(filepath.Join(config.GetAddonDir(), "addons.txt"))
		if readErr != nil {
			logger.Error("SortAddonsTxt: failed to re-read addons.txt after failed write:", readErr)
			return writeErr
		}
		installedAddonNames = lines
		return writeErr
	}

	installedAddonNames = ordered
	return nil
}
