package addon

import (
	"ClassicAddonManager/backend/logger"
	"strings"
)

func GetAddons() []Addon {
	names, err := ReadAddonsTxt()
	if err != nil {
		logger.Error("Error reading addons.txt in GetAddons:", err)
		return nil
	}

	addons := make([]Addon, 0, len(names))
	for _, name := range names {
		addon := Addon{
			Name:  name,
			Alias: strings.ReplaceAll(name, "_", " "),
		}

		if local := FindLocalAddonByName(name); local != nil {
			addon = *local
			if addon.Alias == "" {
				addon.Alias = strings.ReplaceAll(name, "_", " ")
			}
		}

		addons = append(addons, addon)
	}
	return addons
}
