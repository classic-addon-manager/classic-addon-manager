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
	url := fmt.Sprintf("%s/addon/%s/unsubscribe", ApiURL, addonName)

	req, err := http.NewRequest("POST", url, nil)
	if err != nil {
		logger.Error("Error creating request:", err)
		return
	}

	req.Header.Set("X-Client", GetApiClientHeader())
	req.Header.Set("X-Token", shared.AuthToken)
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		logger.Error("Error sending request:", err)
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
	url := fmt.Sprintf("%s/me/addons", ApiURL)

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		logger.Error("Error creating request:", err)
		return nil, err
	}

	req.Header.Set("X-Client", GetApiClientHeader())
	req.Header.Set("X-Token", shared.AuthToken)
	client := &http.Client{}
	resp, err := client.Do(req)
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
