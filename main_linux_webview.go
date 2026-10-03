//go:build linux && cgo && !gtk3 && !server

package main

import "os"

func init() {
	if _, err := os.Stat("/sys/module/nvidia"); err != nil {
		return
	}
	if os.Getenv("WEBKIT_DISABLE_DMABUF_RENDERER") != "1" {
		return
	}

	// Wails beta.5 sets this to 1 on NVIDIA during package initialization.
	// Current WebKitGTK requires the DMA-BUF renderer for compositing, so that
	// workaround also disables backdrop filters, even with GPU policy Always.
	// Keep GPU compositing and use shared-memory frame transport instead of
	// NVIDIA DMA-BUF imports. This must run before the first WebView is created.
	_ = os.Setenv("WEBKIT_DISABLE_DMABUF_RENDERER", "0")
	if os.Getenv("WEBKIT_DMABUF_RENDERER_FORCE_SHM") == "" {
		_ = os.Setenv("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1")
	}
}
