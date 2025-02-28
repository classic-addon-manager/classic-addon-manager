package services

import (
	"ClassicAddonManager/backend/api"
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
)

type ApplicationService struct {
}

func (s *ApplicationService) GetVersion() string {
	return shared.Version
}

func (s *ApplicationService) GetLatestRelease() (api.ApplicationRelease, error) {
	release, err := api.GetLatestApplicationRelease()
	if err != nil {
		logger.Error("Error getting latest application release:", err)
		return api.ApplicationRelease{}, err
	}
	return release, nil
}
