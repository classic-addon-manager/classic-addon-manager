//go:build linux && cgo && !gtk3 && !android && !server

package main

/*
#cgo pkg-config: gtk4
#include <gtk/gtk.h>

// Wails' Frameless calls gtk_window_set_decorated(FALSE) and forces
// border-radius: 0, so GTK draws neither the client-side shadow nor the rounded
// corners, and Wayland compositors don't decorate GTK4 windows themselves.
// An invisible custom titlebar keeps GTK's client-side decorations (shadow,
// rounded corners, resize margins) without drawing a title bar over the
// frontend's own one. Must run before the window is realized.
// Themes such as Breeze only round the top corners, so round all four like
// Windows 11 (8px) and square them again when maximized, fullscreen or tiled.
static const char *cam_window_css =
	"window.cam-window.csd { border-radius: 8px; }"
	"window.cam-window.csd.maximized, window.cam-window.csd.fullscreen,"
	"window.cam-window.csd.tiled, window.cam-window.csd.tiled-top,"
	"window.cam-window.csd.tiled-bottom, window.cam-window.csd.tiled-left,"
	"window.cam-window.csd.tiled-right { border-radius: 0; }";

static gboolean cam_enable_client_side_decorations(void *ptr) {
	GtkWindow *window = GTK_WINDOW(ptr);
	if (gtk_widget_get_realized(GTK_WIDGET(window))) {
		return FALSE;
	}

	GtkWidget *titlebar = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
	gtk_widget_set_visible(titlebar, FALSE);
	gtk_window_set_titlebar(window, titlebar);
	gtk_widget_remove_css_class(GTK_WIDGET(window), "wails-frameless");
	gtk_widget_add_css_class(GTK_WIDGET(window), "cam-window");
	gtk_window_set_decorated(window, TRUE);

	GtkCssProvider *provider = gtk_css_provider_new();
	gtk_css_provider_load_from_string(provider, cam_window_css);
	gtk_style_context_add_provider_for_display(
		gtk_widget_get_display(GTK_WIDGET(window)),
		GTK_STYLE_PROVIDER(provider),
		GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);
	g_object_unref(provider);
	return TRUE;
}
*/
import "C"

import (
	"ClassicAddonManager/backend/logger"
	"sync"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

// prepareWindowChrome keeps the frameless window hidden until GTK's
// client-side decorations are enabled, because they can't be added to a
// realized window. The returned hook must be called with the created window.
func prepareWindowChrome(options *application.WebviewWindowOptions) func(*application.WebviewWindow) {
	if !options.Frameless || options.Hidden {
		return func(*application.WebviewWindow) {}
	}
	options.Hidden = true

	return func(window *application.WebviewWindow) {
		var once sync.Once
		window.OnWindowEvent(events.Linux.WindowLoadFinished, func(*application.WindowEvent) {
			once.Do(func() {
				application.InvokeSync(func() {
					native := window.NativeWindow()
					if native == nil || C.cam_enable_client_side_decorations(native) == 0 {
						logger.Error("Could not enable window shadow: window already realized")
					}
				})
				window.Show()
			})
		})
	}
}
