package util

import (
	"ClassicAddonManager/backend/config"
	"os"
	"regexp"
	"strings"
)

type LogParseResult struct {
	Type  string
	Addon string
	File  string
	Error string
}

func parseLogFile() ([]LogParseResult, error) {
	logPath := config.GetAACDir() + "/ArcheAge.log"
	data, err := os.ReadFile(logPath)
	if err != nil {
		return nil, err
	}

	luaRegex, err := regexp.Compile(`\[Lua Error\].*/Addon/(.+\.lua:\d+): (.+)`)
	if err != nil {
		return nil, err
	}
	addonRegex, err := regexp.Compile(`\/Addon\/([^:]+:\d+):\s*\[Script Error\]\s*(.*)`)
	if err != nil {
		return nil, err
	}
	apiRegex, err := regexp.Compile(`@.*?/x2ui/addons/([^:]+:\d+):\s*\[Script Error\]\s*(.*)`)
	if err != nil {
		return nil, err
	}

	lines := strings.Split(string(data), "\n")

	var results = make([]LogParseResult, 0)

	for _, line := range lines {
		if luaRegex.MatchString(line) {
			matches := luaRegex.FindStringSubmatch(line)
			addonName := strings.Split(matches[1], "/")
			results = append(results, LogParseResult{
				Type:  "lua",
				Addon: addonName[0],
				File:  matches[1],
				Error: matches[2],
			})
		}
		if addonRegex.MatchString(line) {
			matches := addonRegex.FindStringSubmatch(line)
			addonName := strings.Split(matches[1], "/")
			results = append(results, LogParseResult{
				Type:  "addon",
				Addon: addonName[0],
				File:  matches[1],
				Error: matches[2],
			})
		}
		if apiRegex.MatchString(line) {
			matches := apiRegex.FindStringSubmatch(line)
			results = append(results, LogParseResult{
				Type:  "api",
				Addon: "x2ui",
				File:  matches[1],
				Error: matches[2],
			})
		}
	}

	return results, nil
}

func DiagnoseIssues() ([]LogParseResult, error) {
	return parseLogFile()
}
