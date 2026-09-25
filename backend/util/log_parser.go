package util

import (
	"ClassicAddonManager/backend/config"
	"bufio"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

const (
	maxLogTailBytes    int64 = 8 << 20
	maxLogLineBytes          = 64 << 10
	maxLogUniqueErrors       = 5000
)

type LogParseResult struct {
	Type  string
	Addon string
	File  string
	Error string
	Count int
}

// Patterns with their corresponding type and addon name extraction logic
var logPatterns = []struct {
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

func parseLogFile() ([]LogParseResult, error) {
	aacDir, err := config.GetAACDir()
	if err != nil {
		return nil, err
	}
	return parseLogPath(filepath.Join(aacDir, "ArcheAge.log"))
}

func parseLogPath(path string) ([]LogParseResult, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	skipFirst := false
	info, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if info.Size() > maxLogTailBytes {
		if _, err := f.Seek(info.Size()-maxLogTailBytes-1, io.SeekStart); err != nil {
			return nil, err
		}
		skipFirst = true
	}

	return parseLog(f, skipFirst)
}

func parseLog(r io.Reader, skipFirst bool) ([]LogParseResult, error) {
	type logKey struct {
		Type  string
		File  string
		Error string
	}
	type logEntry struct {
		result   LogParseResult
		lastSeen int
	}

	br := bufio.NewReaderSize(r, maxLogLineBytes)
	entries := make(map[logKey]*logEntry)
	seq := 0
	var line []byte

	for {
		line = line[:0]
		var readErr error
		for {
			frag, isPrefix, err := br.ReadLine()
			if rem := maxLogLineBytes - len(line); rem > 0 {
				if len(frag) > rem {
					frag = frag[:rem]
				}
				line = append(line, frag...)
			}
			readErr = err
			if !isPrefix || err != nil {
				break
			}
		}
		if readErr != nil && readErr != io.EOF {
			return nil, readErr
		}
		if n := len(line); n > 0 && line[n-1] == '\r' {
			line = line[:n-1]
		}

		if skipFirst {
			skipFirst = false
		} else if len(line) > 0 {
			for _, p := range logPatterns {
				if matches := p.regex.FindStringSubmatch(string(line)); matches != nil {
					seq++
					key := logKey{Type: p.errType, File: matches[1], Error: matches[2]}
					if e, ok := entries[key]; ok {
						e.result.Count++
						e.lastSeen = seq
					} else {
						entries[key] = &logEntry{
							result: LogParseResult{
								Type:  p.errType,
								Addon: p.getAddon(matches[1]),
								File:  matches[1],
								Error: matches[2],
								Count: 1,
							},
							lastSeen: seq,
						}
					}
					break
				}
			}
		}

		if readErr == io.EOF {
			break
		}
	}

	if len(entries) == 0 {
		return nil, nil
	}
	ordered := make([]*logEntry, 0, len(entries))
	for _, e := range entries {
		ordered = append(ordered, e)
	}
	sort.Slice(ordered, func(i, j int) bool { return ordered[i].lastSeen < ordered[j].lastSeen })
	if len(ordered) > maxLogUniqueErrors {
		ordered = ordered[len(ordered)-maxLogUniqueErrors:]
	}
	results := make([]LogParseResult, len(ordered))
	for i, e := range ordered {
		results[i] = e.result
	}
	return results, nil
}

func DiagnoseIssues() ([]LogParseResult, error) {
	return parseLogFile()
}
