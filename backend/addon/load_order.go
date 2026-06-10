package addon

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"

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
// the addons it (transitively) depends on. dependenciesByName maps an addon
// name to the names it depends on. Only dependency edges whose target is also
// present in names are followed, so the result is a reordering of the provided
// names and never adds or removes entries.
// A circular dependency yields an error and no ordering.
func resolveLoadOrder(names []string, dependenciesByName map[string][]string) ([]string, error) {
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

		for _, dep := range dependenciesByName[name] {
			if _, installed := present[dep]; !installed {
				continue
			}
			if err := visit(dep, append(path, name)); err != nil {
				return err
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
// addons that depend on them, fully resolving transitive dependencies.
// Dependency information is taken from the locally managed addons.
// It only reorders addons that are already listed,
// it does not add missing dependencies. On a circular dependency the file
// is left untouched, the error is logged, and it is returned to the caller.
func SortAddonsTxt() error {
	installedAddonNamesMu.Lock()
	defer installedAddonNamesMu.Unlock()

	addonsTxtPath := filepath.Join(config.GetAddonDir(), "addons.txt")

	// Always read the current file fresh from disk so we sort exactly what is
	// on disk, not a potentially stale in-memory cache.
	lines, err := file.ReadLines(addonsTxtPath)
	if err != nil {
		logger.Error("SortAddonsTxt: failed to read addons.txt:", err)
		return err
	}

	names := make([]string, 0, len(lines))
	for _, line := range lines {
		if strings.TrimSpace(line) == "" {
			continue
		}
		names = append(names, line)
	}

	if len(names) == 0 {
		installedAddonNames = names
		return nil
	}

	// Dependency information comes from the locally managed addons
	// (managed_addons.json), populated in the LocalAddons map.
	dependenciesByName := make(map[string][]string, len(LocalAddons))
	for name, managed := range LocalAddons {
		dependenciesByName[name] = managed.Dependencies
	}

	ordered, err := resolveLoadOrder(names, dependenciesByName)
	if err != nil {
		logger.Error("SortAddonsTxt: failed to resolve load order:", err)
		return err
	}

	if slices.Equal(ordered, names) {
		installedAddonNames = ordered
		return nil
	}

	if writeErr := file.WriteLines(addonsTxtPath, ordered); writeErr != nil {
		logger.Error("SortAddonsTxt: failed to write addons.txt:", writeErr)
		// Rollback the in-memory cache from disk if the write failed.
		rollback, readErr := file.ReadLines(addonsTxtPath)
		if readErr != nil {
			logger.Error("SortAddonsTxt: failed to re-read addons.txt after failed write:", readErr)
			return writeErr
		}
		installedAddonNames = rollback
		return writeErr
	}

	installedAddonNames = ordered
	return nil
}
