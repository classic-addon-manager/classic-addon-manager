//go:build server

package nativeui

import (
	"fmt"
	"os"
)

// HandleNativeErrorArgs handles the hidden same-binary dialog child mode. In
// server mode there is no dialog to spawn, so the message goes to stderr.
func HandleNativeErrorArgs(args []string) bool {
	title, message, ok := parseNativeErrorArgs(args)
	if !ok {
		return false
	}

	ShowFatalError(title, message)
	return true
}

// ShowFatalError reports the error on stderr; server mode has no GUI to block on.
func ShowFatalError(title, message string) {
	_, _ = fmt.Fprintf(os.Stderr, "%s: %s\n", title, message)
}
