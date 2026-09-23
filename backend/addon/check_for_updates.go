package addon

import (
	_ "embed"
	"os"
	"path/filepath"
	"strings"
	"sync"

	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/logger"
)

//go:embed cam.lua
var luaScript []byte

func CheckForUpdates() map[string]Addon {
	err := LoadManagedAddonsFile()
	if err != nil {
		logger.Error("Error loading managed addons file:", err)
	}

	var (
		wg      sync.WaitGroup
		mu      sync.Mutex
		updates = make(map[string]Addon)
	)

	for _, addon := range localAddonsSnapshot() {
		wg.Add(1)
		go func(a Addon) {
			defer wg.Done()

			release, err := api.GetAddonRelease(a.Name, "latest")
			if err != nil {
				logger.Error("Error getting latest release for "+a.Name+":", err)
				return
			}

			mu.Lock()
			defer mu.Unlock()

			if release.TagName != a.Version {
				updatedAddon := a
				updatedAddon.Version = release.TagName
				updates[a.Name] = updatedAddon
			}
		}(addon)
	}

	wg.Wait()
	return updates
}

func GenerateUpdateAddonLua(updates map[string]Addon) {
	addonPath := filepath.Join(config.GetAddonDir(), "AddonUpdateNotification")

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

	// Remove old file if it exists
	err := os.Remove(filepath.Join(addonPath, "main.lua"))
	if err != nil && !os.IsNotExist(err) {
		logger.Error("Error removing old AddonUpdateNotification main.lua:", err)
		return
	}

	// Write new file
	file, err := os.Create(filepath.Join(addonPath, "main.lua"))
	if err != nil {
		logger.Error("Error creating AddonUpdateNotification main.lua:", err)
		return
	}
	defer file.Close()

	written, err := file.Write(luaScript)
	if err != nil {
		logger.Error("Error writing AddonUpdateNotification main.lua:", err)
		return
	}
	if written != len(luaScript) {
		logger.Error("Error writing AddonUpdateNotification main.lua: Not all bytes written", nil)
		return
	}

	err = os.Remove(filepath.Join(addonPath, "updates.lua"))
	if err != nil && !os.IsNotExist(err) {
		logger.Error("Error removing old AddonUpdateNotification updates.lua:", err)
		return
	}

	if len(updates) == 0 {
		return
	}

	file, err = os.Create(filepath.Join(addonPath, "updates.lua"))
	if err != nil {
		logger.Error("Error creating AddonUpdateNotification updates.lua:", err)
		return
	}
	defer file.Close()

	_, err = file.Write(generateUpdatesLua(updates))
	if err != nil {
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
