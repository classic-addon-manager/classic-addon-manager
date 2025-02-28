package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/logger"
)

type RemoteAddonService struct{}

func (s *RemoteAddonService) GetAddonManifest() []addon.AddonManifest {
	return addon.GetAddonManifest()
}

func (s *RemoteAddonService) InstallAddon(ad addon.AddonManifest) (bool, error) {
	_, err := addon.InstallAddon(ad)
	if err != nil {
		logger.Error("Error installing addon:", err)
		return false, err
	}

	ok := addon.AddToAddonsTxt(ad.Name)
	if !ok {
		logger.Warn(ad.Name + " - Error adding addon to addons.txt")
	}

	return ok, nil
}
