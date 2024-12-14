package util

import (
	"encoding/json"
	"io"
	"log"
	"os"
)

var PackageJSON struct {
	Version string `json:"version"`
}

func init() {
	file, err := os.Open("frontend/package.json")
	if err != nil {
		log.Fatalf("failed to read package.json: %v", err)
	}
	defer file.Close()

	byteValue, _ := io.ReadAll(file)
	err = json.Unmarshal(byteValue, &PackageJSON)
	if err != nil {
		log.Fatalf("failed to parse package.json: %v", err)
	}
}

func GetVersion() string {
	return PackageJSON.Version
}
