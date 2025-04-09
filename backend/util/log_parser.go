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

	// Define patterns with their corresponding type and addon name extraction logic
	patterns := []struct {
		regex    *regexp.Regexp
		errType  string
		getAddon func(string) string
	}{
		{
			regex:    regexp.MustCompile(`\[Lua Error].*/Addon/(.+\.lua:\d+): (.+)`),
			errType:  "lua",
			getAddon: func(file string) string { return strings.Split(file, "/")[0] },
		},
		{
			regex:    regexp.MustCompile(`/Addon/([^:]+:\d+):\s*\[Script Error]\s*(.*)`),
			errType:  "addon",
			getAddon: func(file string) string { return strings.Split(file, "/")[0] },
		},
		{
			regex:    regexp.MustCompile(`@.*?/x2ui/addons/([^:]+:\d+):\s*\[Script Error]\s*(.*)`),
			errType:  "api",
			getAddon: func(file string) string { return "x2ui" },
		},
	}

	lines := strings.Split(string(data), "\n")
	var results []LogParseResult

	for _, line := range lines {
		for _, p := range patterns {
			if matches := p.regex.FindStringSubmatch(line); matches != nil {
				results = append(results, LogParseResult{
					Type:  p.errType,
					Addon: p.getAddon(matches[1]),
					File:  matches[1],
					Error: matches[2],
				})
				break
			}
		}
	}

	return results, nil
}

func DiagnoseIssues() ([]LogParseResult, error) {
	return parseLogFile()
}
