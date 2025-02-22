package api

import "ClassicAddonManager/backend/shared"

const ApiURL = "https://aac.gaijin.dev"

func GetApiClientHeader() string {
	return "Classic Addon Manager " + shared.Version
}
