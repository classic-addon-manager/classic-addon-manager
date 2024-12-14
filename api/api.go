package api

import "ClassicAddonManager/util"

const apiUrl = "https://aac.gaijin.dev"

func GetApiClientHeader() string {
	return "Classic Addon Manager " + util.GetVersion()
}
