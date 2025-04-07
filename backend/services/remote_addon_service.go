package services

import (
	"ClassicAddonManager/backend/addon"
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
)

type RemoteAddonService struct{}

func (s *RemoteAddonService) GetAddonManifest() []shared.AddonManifest {
	return addon.GetAddonManifest()
}

func (s *RemoteAddonService) InstallAddon(ad shared.AddonManifest) (bool, error) {
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

func (s *RemoteAddonService) GetLatestRelease(name string) (api.Release, error) {
	release, err := api.GetLatestAddonRelease(name)
	if err != nil {
		logger.Error("Error getting latest release:", err)
		return api.Release{}, err
	}
	return release, nil
}

func (s *RemoteAddonService) GetSubscribedAddons() ([]shared.AddonManifest, error) {
	return api.GetSubscribedAddons()
}
