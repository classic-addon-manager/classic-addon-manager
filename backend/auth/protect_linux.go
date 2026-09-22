//go:build linux

package auth

import (
	"fmt"
)

func protectToken(plainToken string) (string, string, error) {
	return plainToken, "plain", nil
}

func unprotectToken(encoding, token string) (string, error) {
	if encoding == "plain" {
		return token, nil
	}
	return "", fmt.Errorf("unsupported encoding on linux: %s", encoding)
}
