//go:build linux

package auth

import (
	"fmt"
	"os"
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

func afterWriteHook(path string) error {
	if err := os.Chmod(path, 0600); err != nil {
		return fmt.Errorf("error setting file permissions: %w", err)
	}
	return nil
}
