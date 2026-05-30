package api

import (
	"ClassicAddonManager/backend/logger"
	"ClassicAddonManager/backend/shared"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"strconv"
)

func UnsubscribeFromAddon(addonName string) {
	path := fmt.Sprintf("/addon/%s/unsubscribe", addonName)

	req, err := newAuthenticatedApiRequest(nil, http.MethodPost, path, nil)
	if err != nil {
		logger.Error("Error creating request:", err)
		return
	}

	resp, err := apiClient.Do(req)
	if err != nil {
		logger.Error("Error creating request:", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		logger.Error("Error unsubscribing from addon:", errors.New(strconv.Itoa(resp.StatusCode)))
		return
	}

	logger.Info("Unsubscribed from addon: " + addonName)
}

type SubscribedAddonsResponse struct {
	Addons []shared.AddonManifest `json:"data"`
}

func GetSubscribedAddons() ([]shared.AddonManifest, error) {
	req, err := newAuthenticatedApiRequest(nil, http.MethodGet, "/me/addons", nil)
	if err != nil {
		logger.Error("Error creating request:", err)
		return nil, err
	}

	resp, err := apiClient.Do(req)
	if err != nil {
		logger.Error("Error sending request:", err)
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		logger.Error("Error getting subscribed addons:", errors.New(strconv.Itoa(resp.StatusCode)))
		return nil, errors.New(strconv.Itoa(resp.StatusCode))
	}

	var response SubscribedAddonsResponse
	err = json.NewDecoder(resp.Body).Decode(&response)
	if err != nil {
		logger.Error("Error decoding response:", err)
		return nil, err
	}

	return response.Addons, nil
}
