package addon

import (
	"fmt"
	"strings"
)

func GetAddons() []Addon {
	addons := ReadAddonsTxt()
	var addonsList = make([]Addon, 0)
	for _, ad := range addons {
		addon := FindLocalAddonByName(ad)
		fmt.Println("Addon:", ad)
		if addon != nil {
			fmt.Println("Addon found:", addon)
			addonsList = append(addonsList, *addon)
			continue
		}

		addonsList = append(addonsList, Addon{
			Name:        ad,
			DisplayName: strings.ReplaceAll(ad, "_", " "),
			Version:     "",
			Commit:      "",
			Author:      "",
			Repo:        "",
			IsManaged:   false,
		})
	}
	return addonsList
}
