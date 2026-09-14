// Command setversion updates the application version in source and build files.
package main

import (
	"flag"
	"fmt"
	"os"
	"regexp"
	"strconv"
	"strings"
)

type target struct {
	path     string
	patterns []*regexp.Regexp
}

type loaded struct {
	target
	content  string
	versions []string
}

var versionRe = regexp.MustCompile(`^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$`)

func main() {
	set := flag.String("set", "", "set the version explicitly, e.g. -set 3.3.0")
	var bump string
	flag.Func("bump", "bump the version: major, minor, or patch", func(v string) error {
		switch v {
		case "major", "minor", "patch":
			bump = v
			return nil
		}
		return fmt.Errorf("invalid -bump %q: must be major, minor, or patch", v)
	})
	flag.Parse()

	if flag.NArg() > 0 {
		fatal("unexpected positional arguments: %s", strings.Join(flag.Args(), " "))
	}
	if (*set == "") == (bump == "") {
		fatal("exactly one of -set X.Y.Z or -bump major|minor|patch is required")
	}
	if _, err := os.Stat("go.mod"); err != nil {
		fatal("cannot read go.mod. Run from the project root: %v", err)
	}

	newVersion := strings.TrimPrefix(*set, "v")
	if *set != "" && !versionRe.MatchString(newVersion) {
		fatal("invalid version %q: expected X.Y.Z (e.g. 3.3.0)", *set)
	}

	// Each pattern captures the prefix, full version value, and closing quote.
	// Match horizontal whitespace so indentation cannot consume a newline.
	targets := []target{
		{"build/config.yml", []*regexp.Regexp{
			regexp.MustCompile(`(?m)^([ \t]*version: ")([^"]*)(")`),
		}},
		{"build/windows/info.json", []*regexp.Regexp{
			regexp.MustCompile(`("file_version": ")([^"]*)(")`),
			regexp.MustCompile(`("ProductVersion": ")([^"]*)(")`),
		}},
		{"build/windows/wails.exe.manifest", []*regexp.Regexp{
			regexp.MustCompile(`(?m)^([ \t]*<assemblyIdentity [^>]*name="dev\.gaijin\.classicaddonmanager"[^>]*version=")([^"]*)(")`),
		}},
		// Update only the version. Regenerating assets can overwrite build settings.
		{"build/windows/nsis/wails_tools.nsh", []*regexp.Regexp{
			regexp.MustCompile(`(?m)^([ \t]*!define INFO_PRODUCTVERSION ")([^"]*)(")`),
		}},
		{"build/linux/nfpm/nfpm.yaml", []*regexp.Regexp{
			regexp.MustCompile(`(?m)^(version: ")([^"]*)(")`),
		}},
		{"backend/shared/types.go", []*regexp.Regexp{
			regexp.MustCompile(`(?m)^(var Version = ")([^"]*)(")`),
		}},
	}

	// Check every field before writing. SET can repair invalid or mismatched values.
	files := make([]loaded, 0, len(targets))
	for _, t := range targets {
		b, err := os.ReadFile(t.path)
		if err != nil {
			fatal("read %s: %v", t.path, err)
		}
		f := loaded{target: t, content: string(b)}
		for _, pattern := range t.patterns {
			matches := pattern.FindAllStringSubmatch(f.content, -1)
			if len(matches) != 1 {
				fatal("%s: expected one match for %s, found %d", t.path, pattern, len(matches))
			}
			v := matches[0][2]
			if bump != "" && !versionRe.MatchString(v) {
				fatal("%s: invalid version %q. Use -set X.Y.Z to fix it", t.path, v)
			}
			f.versions = append(f.versions, v)
		}
		files = append(files, f)
	}

	oldVersion := files[0].versions[0]
	if bump != "" {
		if !allAgree(files, oldVersion) {
			fmt.Fprintln(os.Stderr, "version drift detected:")
			for _, f := range files {
				fmt.Fprintf(os.Stderr, "  %s: %s\n", f.path, strings.Join(f.versions, ", "))
			}
			fatal("use -set X.Y.Z to give all files the same version")
		}
		newVersion = bumpVersion(oldVersion, bump)
	}

	if allAgree(files, newVersion) {
		fmt.Printf("version already %s, nothing to do\n", newVersion)
		return
	}

	for _, f := range files {
		next := f.content
		for _, pattern := range f.patterns {
			next = pattern.ReplaceAllString(next, `${1}`+newVersion+`${3}`)
		}
		if next == f.content {
			continue
		}
		if err := os.WriteFile(f.path, []byte(next), 0o644); err != nil {
			fatal("write %s: %v. Some files may already be updated. Resume with -set %s", f.path, err, newVersion)
		}
		fmt.Printf("  %s: %s -> %s\n", f.path, strings.Join(f.versions, ", "), newVersion)
	}
	fmt.Printf("version is now %s across %d files\n", newVersion, len(files))
}

func allAgree(files []loaded, want string) bool {
	for _, f := range files {
		for _, v := range f.versions {
			if v != want {
				return false
			}
		}
	}
	return true
}

func bumpVersion(v, part string) string {
	parts := strings.Split(v, ".")
	index := 2
	switch part {
	case "major":
		index = 0
	case "minor":
		index = 1
	}
	n, err := strconv.Atoi(parts[index])
	if err != nil || n == int(^uint(0)>>1) {
		fatal("cannot bump %q: %s component is too large", v, part)
	}
	parts[index] = strconv.Itoa(n + 1)
	for i := index + 1; i < len(parts); i++ {
		parts[i] = "0"
	}
	return strings.Join(parts, ".")
}

func fatal(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "setversion: "+format+"\n", args...)
	os.Exit(1)
}
