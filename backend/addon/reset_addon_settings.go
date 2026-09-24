package addon

import (
	"ClassicAddonManager/backend/config"
	"os"
	"path/filepath"
)

func ResetAddonSettings() error {
	aacDir, err := config.GetAACDir()
	if err != nil {
		return err
	}
	return os.Truncate(filepath.Join(aacDir, "addon_settings"), 0)
}
