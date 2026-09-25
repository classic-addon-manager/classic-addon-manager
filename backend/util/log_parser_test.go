package util

import (
	"bytes"
	"fmt"
	"path/filepath"
	"strings"
	"testing"
)

func TestParseLogErrorTypes(t *testing.T) {
	input := strings.Join([]string{
		"[Lua Error] foo/Addon/MyAddon/main.lua:12: attempt to index nil",
		"x/Addon/MyAddon/ui.lua:5: [Script Error] bad thing",
		"@game/x2ui/addons/bag.lua:7: [Script Error] api err",
		"noise line that matches nothing",
	}, "\r\n") + "\r\n"

	results, err := parseLog(strings.NewReader(input), false)
	if err != nil {
		t.Fatalf("parseLog: %v", err)
	}
	if len(results) != 3 {
		t.Fatalf("got %d results, want 3: %v", len(results), results)
	}

	want := []LogParseResult{
		{Type: "lua", Addon: "MyAddon", File: "MyAddon/main.lua:12", Error: "attempt to index nil", Count: 1},
		{Type: "addon", Addon: "MyAddon", File: "MyAddon/ui.lua:5", Error: "bad thing", Count: 1},
		{Type: "api", Addon: "x2ui", File: "bag.lua:7", Error: "api err", Count: 1},
	}
	for i, w := range want {
		if results[i] != w {
			t.Fatalf("results[%d] = %+v, want %+v", i, results[i], w)
		}
	}
}

func TestParseLogOverlongLine(t *testing.T) {
	input := strings.Repeat("x", 200<<10) + "\n" +
		"x/Addon/MyAddon/ui.lua:5: [Script Error] bad thing\n"

	results, err := parseLog(strings.NewReader(input), false)
	if err != nil {
		t.Fatalf("parseLog: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("got %d results, want 1: %v", len(results), results)
	}
	if results[0].File != "MyAddon/ui.lua:5" || results[0].Error != "bad thing" {
		t.Fatalf("results[0] = %+v", results[0])
	}
}

func TestParseLogMergesRepeatedErrors(t *testing.T) {
	line := "x/Addon/MyAddon/ui.lua:5: [Script Error] bad thing"
	input := line + "\n" + line + "\n" + line + "\n" +
		"x/Addon/MyAddon/ui.lua:5: [Script Error] other thing\n"

	results, err := parseLog(strings.NewReader(input), false)
	if err != nil {
		t.Fatalf("parseLog: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("got %d results, want 2: %v", len(results), results)
	}
	counts := map[string]int{}
	for _, r := range results {
		counts[r.Error] = r.Count
	}
	if counts["bad thing"] != 3 {
		t.Fatalf("bad thing count = %d, want 3", counts["bad thing"])
	}
	if counts["other thing"] != 1 {
		t.Fatalf("other thing count = %d, want 1", counts["other thing"])
	}
}

func TestParseLogOrdersByLatestOccurrence(t *testing.T) {
	input := "x/Addon/A/a.lua:1: [Script Error] first\n" +
		"x/Addon/B/b.lua:2: [Script Error] second\n" +
		"x/Addon/A/a.lua:1: [Script Error] first\n"

	results, err := parseLog(strings.NewReader(input), false)
	if err != nil {
		t.Fatalf("parseLog: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("got %d results, want 2: %v", len(results), results)
	}
	if results[0].Error != "second" || results[1].Error != "first" {
		t.Fatalf("result order = %q, %q, want second, first", results[0].Error, results[1].Error)
	}
	if results[1].Count != 2 {
		t.Fatalf("last result count = %d, want 2", results[1].Count)
	}
}

func TestParseLogResultCap(t *testing.T) {
	var buf bytes.Buffer
	for i := 0; i < maxLogUniqueErrors+100; i++ {
		fmt.Fprintf(&buf, "x/Addon/A/f.lua:1: [Script Error] err %d\n", i)
	}

	results, err := parseLog(&buf, false)
	if err != nil {
		t.Fatalf("parseLog: %v", err)
	}
	if len(results) != maxLogUniqueErrors {
		t.Fatalf("got %d results, want %d", len(results), maxLogUniqueErrors)
	}
	if got, want := results[0].Error, "err 100"; got != want {
		t.Fatalf("first result error = %q, want %q", got, want)
	}
	if got, want := results[len(results)-1].Error, "err 5099"; got != want {
		t.Fatalf("last result error = %q, want %q", got, want)
	}
}

func TestParseLogPathKeepsLineStartingAtTailBoundary(t *testing.T) {
	filler := strings.Repeat("x", 100) + "\n"
	tailSize := int(maxLogTailBytes)

	var tail strings.Builder
	tail.WriteString("x/Addon/Edge/e.lua:3: [Script Error] boundary error\n")
	for tail.Len()+len(filler) <= tailSize {
		tail.WriteString(filler)
	}
	if rem := tailSize - tail.Len(); rem > 0 {
		tail.WriteString(strings.Repeat("x", rem-1) + "\n")
	}
	if tail.Len() != tailSize {
		t.Fatalf("tail length = %d, want %d", tail.Len(), tailSize)
	}

	path := filepath.Join(t.TempDir(), "ArcheAge.log")
	writeTestFile(t, path, strings.Repeat(filler, 3)+tail.String())

	results, err := parseLogPath(path)
	if err != nil {
		t.Fatalf("parseLogPath: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("got %d results, want 1: %v", len(results), results)
	}
	if got, want := results[0].Error, "boundary error"; got != want {
		t.Fatalf("results[0].Error = %q, want %q", got, want)
	}
}

func TestParseLogPathDropsPartialLineAtTailBoundary(t *testing.T) {
	filler := strings.Repeat("x", 100) + "\n"
	freshLine := "x/Addon/New/n.lua:2: [Script Error] fresh error\n"
	tailSize := int(maxLogTailBytes)

	var tail strings.Builder
	tail.WriteString("xxxxx/Addon/Cut/c.lua:1: [Script Error] partial error\n")
	for tail.Len()+len(filler) <= tailSize-len(freshLine) {
		tail.WriteString(filler)
	}
	if rem := tailSize - len(freshLine) - tail.Len(); rem > 0 {
		tail.WriteString(strings.Repeat("x", rem-1) + "\n")
	}
	tail.WriteString(freshLine)
	if tail.Len() != tailSize {
		t.Fatalf("tail length = %d, want %d", tail.Len(), tailSize)
	}

	path := filepath.Join(t.TempDir(), "ArcheAge.log")
	writeTestFile(t, path, strings.Repeat(filler, 3)+"xxxxx"+tail.String())

	results, err := parseLogPath(path)
	if err != nil {
		t.Fatalf("parseLogPath: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("got %d results, want 1: %v", len(results), results)
	}
	if got, want := results[0].Error, "fresh error"; got != want {
		t.Fatalf("results[0].Error = %q, want %q", got, want)
	}
}

func TestParseLogPathTailSeek(t *testing.T) {
	var buf bytes.Buffer
	buf.WriteString("x/Addon/Old/old.lua:1: [Script Error] stale error\n")
	for buf.Len() <= int(maxLogTailBytes) {
		buf.WriteString(strings.Repeat("x", 100) + "\n")
	}
	buf.WriteString("x/Addon/New/new.lua:2: [Script Error] fresh error\n")

	path := filepath.Join(t.TempDir(), "ArcheAge.log")
	writeTestFile(t, path, buf.String())

	results, err := parseLogPath(path)
	if err != nil {
		t.Fatalf("parseLogPath: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("got %d results, want 1: %v", len(results), results)
	}
	if got, want := results[0].Error, "fresh error"; got != want {
		t.Fatalf("results[0].Error = %q, want %q", got, want)
	}
}
