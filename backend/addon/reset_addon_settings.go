package addon

import (
	"ClassicAddonManager/backend/config"
	"os"
)

func ResetAddonSettings() error {
	return os.Truncate(config.GetAddonDir()+"/addon_settings", 0)
}
