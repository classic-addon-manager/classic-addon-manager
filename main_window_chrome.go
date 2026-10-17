//go:build !(linux && cgo && !gtk3 && !android && !server)

package main

import "github.com/wailsapp/wails/v3/pkg/application"

// prepareWindowChrome is a no-op where the platform already decorates frameless
// windows (Windows keeps the DWM shadow and rounded corners).
func prepareWindowChrome(*application.WebviewWindowOptions) func(*application.WebviewWindow) {
	return func(*application.WebviewWindow) {}
}
