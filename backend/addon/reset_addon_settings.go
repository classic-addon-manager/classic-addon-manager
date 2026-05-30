package addon

import (
	"ClassicAddonManager/backend/config"
	"os"
	"path/filepath"
)

func ResetAddonSettings() error {
	return os.Truncate(filepath.Join(config.GetAACDir(), "addon_settings"), 0)
}
