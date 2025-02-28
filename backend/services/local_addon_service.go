package services

import "ClassicAddonManager/backend/addon"

type LocalAddonService struct{}

func (s *LocalAddonService) GetAddOns() []addon.Addon {
	return addon.GetAddons()
}
