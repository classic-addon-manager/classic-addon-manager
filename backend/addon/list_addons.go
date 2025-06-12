package addon

import (
	"fmt"
	"strings"
)

func GetAddons() []Addon {
	var addons []Addon
	for _, name := range ReadAddonsTxt() {
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
