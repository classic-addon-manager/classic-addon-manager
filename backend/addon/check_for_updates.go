package addon

import (
	_ "embed"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strings"

	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/file"
	"ClassicAddonManager/backend/logger"
)

//go:embed cam.lua
var luaScript []byte

// CheckForUpdates returns the managed addons with a newer release available.
// A nil map signals a total failure: the caller must not regenerate the
// update notification and lose the previously reported updates.
func CheckForUpdates() (map[string]Addon, error) {
	if err := LoadManagedAddonsFile(); err != nil {
		return nil, fmt.Errorf("load managed addons: %w", err)
	}

	addons := localAddonsSnapshot()
	updates := make(map[string]Addon)
	if len(addons) == 0 {
		return updates, nil
	}

	names := make([]string, 0, len(addons))
	for name := range addons {
		names = append(names, name)
	}
	slices.Sort(names)

	releases, err := api.GetLatestReleasesBulk(names)
	if err != nil {
		return nil, fmt.Errorf("fetch latest releases: %w", err)
	}

	var missing []string
	for name, a := range addons {
		release, ok := releases[name]
		if !ok || release.TagName == "" {
			missing = append(missing, name)
			continue
		}
		if release.TagName != a.Version {
			updatedAddon := a
			updatedAddon.Version = release.TagName
			updates[name] = updatedAddon
		}
	}

	if len(missing) > 0 {
		slices.Sort(missing)
		return updates, fmt.Errorf("no release info for: %s", strings.Join(missing, ", "))
	}

	return updates, nil
}

func GenerateUpdateAddonLua(updates map[string]Addon) {
	addonDir, err := config.GetAddonDir()
	if err != nil {
		logger.Error("Error resolving addon directory:", err)
		return
	}
	addonPath := filepath.Join(addonDir, "AddonUpdateNotification")

	if _, err := os.Stat(addonPath); os.IsNotExist(err) {
		err = os.MkdirAll(addonPath, os.ModePerm)
		if err != nil {
			logger.Error("Error creating AddonUpdateNotification directory:", err)
			return
		}
	}

	if _, err := ReadAddonsTxt(); err != nil {
		logger.Error("Error reading addons txt:", err)
		return
	}

	if err := AddToAddonsTxt("AddonUpdateNotification"); err != nil {
		logger.Error("Error adding AddonUpdateNotification to addons.txt", err)
		return
	}

	if err := file.WriteAtomic(filepath.Join(addonPath, "main.lua"), luaScript, 0644); err != nil {
		logger.Error("Error writing AddonUpdateNotification main.lua:", err)
		return
	}

	if len(updates) == 0 {
		err := os.Remove(filepath.Join(addonPath, "updates.lua"))
		if err != nil && !os.IsNotExist(err) {
			logger.Error("Error removing old AddonUpdateNotification updates.lua:", err)
		}
		return
	}

	if err := file.WriteAtomic(filepath.Join(addonPath, "updates.lua"), generateUpdatesLua(updates), 0644); err != nil {
		logger.Error("Error writing AddonUpdateNotification updates.lua:", err)
		return
	}
}

func generateUpdatesLua(updates map[string]Addon) []byte {
	var luaTable strings.Builder
	luaTable.WriteString("{\n")

	for _, addon := range updates {
		displayName := addon.Name
		if addon.Alias != "" {
			displayName = addon.Alias
		}
		luaTable.WriteString("    [")
		writeLuaString(&luaTable, addon.Name)
		luaTable.WriteString("] = {name=")
		writeLuaString(&luaTable, displayName)
		luaTable.WriteString(", version=")
		writeLuaString(&luaTable, addon.Version)
		luaTable.WriteString("}, \n")
	}

	luaTable.WriteString("}\n")
	return []byte(luaTable.String())
}

// Lua decimal escapes work in Lua 5.1 and stay unambiguous before digits.
func writeLuaString(b *strings.Builder, value string) {
	b.WriteByte('"')
	for i := range len(value) {
		c := value[i]
		switch {
		case c == '"' || c == '\\':
			b.WriteByte('\\')
			b.WriteByte(c)
		case c < ' ' || c == 0x7f:
			b.WriteByte('\\')
			b.WriteByte('0' + c/100)
			b.WriteByte('0' + c/10%10)
			b.WriteByte('0' + c%10)
		default:
			b.WriteByte(c)
		}
	}
	b.WriteByte('"')
}
