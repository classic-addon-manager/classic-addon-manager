package api

import (
	"ClassicAddonManager/backend/auth"
	"bytes"
	"context"
	"io"
	"net/http"
	"testing"
)

func TestApiRequestHeaders(t *testing.T) {
	const testToken = "test-token-123"

	originalToken := auth.GetToken()
	t.Cleanup(func() { auth.SetToken(originalToken) })

	tests := []struct {
		name            string
		authenticated   bool
		method          string
		body            []byte
		token           string
		wantContentType bool
		wantAccept      bool
		wantClient      bool
		wantToken       bool
	}{
		{
			name:            "GET nil body has no Content-Type",
			method:          http.MethodGet,
			wantContentType: false,
			wantAccept:      true,
			wantClient:      true,
		},
		{
			name:            "POST with body has Content-Type",
			method:          http.MethodPost,
			body:            []byte(`{"key":"value"}`),
			wantContentType: true,
			wantAccept:      true,
			wantClient:      true,
		},
		{
			name:            "authenticated POST nil body keeps Content-Type and sets X-Token",
			authenticated:   true,
			method:          http.MethodPost,
			token:           testToken,
			wantContentType: true,
			wantAccept:      true,
			wantClient:      true,
			wantToken:       true,
		},
		{
			name:            "authenticated GET empty token has no X-Token",
			authenticated:   true,
			method:          http.MethodGet,
			token:           "",
			wantContentType: false,
			wantAccept:      true,
			wantClient:      true,
			wantToken:       false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			auth.SetToken(tt.token)

			var body io.Reader
			if tt.body != nil {
				body = bytes.NewReader(tt.body)
			}

			var req *http.Request
			var err error
			if tt.authenticated {
				req, err = NewAuthenticatedApiRequest(context.Background(), tt.method, "/test", body)
			} else {
				req, err = NewApiRequest(context.Background(), tt.method, "/test", body)
			}
			if err != nil {
				t.Fatalf("request creation failed: %v", err)
			}

			_, hasContentType := req.Header["Content-Type"]
			if hasContentType != tt.wantContentType {
				t.Errorf("Content-Type presence = %v, want %v", hasContentType, tt.wantContentType)
			}
			if tt.wantContentType && req.Header.Get("Content-Type") != "application/json" {
				t.Errorf("Content-Type = %q, want application/json", req.Header.Get("Content-Type"))
			}

			if tt.wantAccept && req.Header.Get("Accept") != "application/json" {
				t.Errorf("Accept = %q, want application/json", req.Header.Get("Accept"))
			}

			if tt.wantClient && req.Header.Get("X-Client") == "" {
				t.Error("X-Client header missing")
			}

			token, hasToken := req.Header["X-Token"]
			if hasToken != tt.wantToken {
				t.Errorf("X-Token presence = %v, want %v", hasToken, tt.wantToken)
			}
			if tt.wantToken && token[0] != tt.token {
				t.Errorf("X-Token = %q, want %q", token[0], tt.token)
			}
		})
	}
}
