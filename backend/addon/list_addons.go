package addon

import (
	"ClassicAddonManager/backend/logger"
	"fmt"
	"strings"
)

func GetAddons() []Addon {
	var addons []Addon
	names, err := ReadAddonsTxt()
	if err != nil {
		logger.Error("Error reading addons.txt in GetAddons:", err)
		return addons
	}

	for _, name := range names {
		fmt.Println("Addon:", name)
		alias := strings.ReplaceAll(name, "_", " ")

		if local := FindLocalAddonByName(name); local != nil {
			fmt.Println("Addon found:", local)
			if local.Alias == "" {
				local.Alias = alias
			}
			addons = append(addons, *local)
		} else {
			addons = append(addons, Addon{
				Name:  name,
				Alias: alias,
			})
		}
	}
	return addons
}
