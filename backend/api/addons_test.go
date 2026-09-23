package api

import (
	"io"
	"net/http"
	"strings"
	"testing"
)

type subscribedAddonsRoundTripper func(*http.Request) (*http.Response, error)

func (roundTrip subscribedAddonsRoundTripper) RoundTrip(req *http.Request) (*http.Response, error) {
	return roundTrip(req)
}

func TestGetSubscribedAddonsEnvelope(t *testing.T) {
	originalClient := Client
	t.Cleanup(func() { Client = originalClient })

	for _, tc := range []struct {
		name    string
		body    string
		wantErr string
	}{
		{
			name:    "API failure",
			body:    `{"status":false,"message":"subscription unavailable","data":[]}`,
			wantErr: "subscription unavailable",
		},
		{
			name: "successful empty list",
			body: `{"status":true,"data":[]}`,
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			Client = &http.Client{Transport: subscribedAddonsRoundTripper(func(req *http.Request) (*http.Response, error) {
				return &http.Response{
					StatusCode: http.StatusOK,
					Body:       io.NopCloser(strings.NewReader(tc.body)),
				}, nil
			})}

			addons, err := GetSubscribedAddons()
			if tc.wantErr != "" {
				if err == nil || err.Error() != tc.wantErr {
					t.Fatalf("expected server error %q, got %v", tc.wantErr, err)
				}
				if addons != nil {
					t.Fatalf("expected no addons on failure, got %v", addons)
				}
				return
			}
			if err != nil {
				t.Fatalf("expected successful empty list, got error %v", err)
			}
			if addons == nil || len(addons) != 0 {
				t.Fatalf("expected decoded empty list, got %v", addons)
			}
		})
	}
}
