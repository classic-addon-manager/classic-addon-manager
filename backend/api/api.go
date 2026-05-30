package api

import (
	"ClassicAddonManager/backend/auth"
	"ClassicAddonManager/backend/shared"
	"context"
	"io"
	"net/http"
	"time"
)

const ApiURL = "https://aac.gaijin.dev"

var apiClient = &http.Client{
	Timeout: time.Second * 30,
}

func newApiRequest(ctx context.Context, method string, path string, body io.Reader) (*http.Request, error) {
	if ctx == nil {
		ctx = context.Background()
	}

	req, err := http.NewRequestWithContext(ctx, method, ApiURL+path, body)
	if err != nil {
		return nil, err
	}

	req.Header.Set("X-Client", GetApiClientHeader())
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Content-Type", "application/json")
	return req, nil
}

func newAuthenticatedApiRequest(ctx context.Context, method string, path string, body io.Reader) (*http.Request, error) {
	req, err := newApiRequest(ctx, method, path, body)
	if err != nil {
		return nil, err
	}

	req.Header.Set("X-Token", auth.GetToken())
	return req, nil
}

func GetApiClientHeader() string {
	return "Classic Addon Manager " + shared.Version
}
