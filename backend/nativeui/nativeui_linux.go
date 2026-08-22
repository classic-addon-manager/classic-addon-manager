//go:build linux

package nativeui

/*
#cgo pkg-config: gtk4
#include <gtk/gtk.h>
#include <stdlib.h>

static void nativeui_alert_done(GObject *source, GAsyncResult *result, gpointer user_data) {
	GError *error = NULL;
	gtk_alert_dialog_choose_finish(GTK_ALERT_DIALOG(source), result, &error);
	if (error != NULL) {
		g_error_free(error);
	}
	g_main_loop_quit((GMainLoop *)user_data);
}

static void nativeui_show_fatal_error(const char *title, const char *message) {
	gtk_init();

	GMainLoop *loop = g_main_loop_new(NULL, FALSE);
	GtkAlertDialog *dialog = gtk_alert_dialog_new("%s", title);
	const char *buttons[] = { "OK", NULL };
	gtk_alert_dialog_set_detail(dialog, message);
	gtk_alert_dialog_set_buttons(dialog, buttons);
	gtk_alert_dialog_set_default_button(dialog, 0);
	gtk_alert_dialog_set_cancel_button(dialog, 0);
	gtk_alert_dialog_set_modal(dialog, TRUE);
	gtk_alert_dialog_choose(dialog, NULL, NULL, nativeui_alert_done, loop);
	g_main_loop_run(loop);

	g_object_unref(dialog);
	g_main_loop_unref(loop);
}
*/
import "C"

import (
	"os"
	"os/exec"
	"unsafe"
)

// HandleNativeErrorArgs handles the hidden same-binary dialog child mode.
func HandleNativeErrorArgs(args []string) bool {
	title, message, ok := parseNativeErrorArgs(args)
	if !ok {
		return false
	}

	showFatalErrorInProcess(title, message)
	return true
}

// ShowFatalError displays a blocking GTK 4 error alert.
func ShowFatalError(title, message string) {
	executable, err := os.Executable()
	if err == nil {
		cmd := exec.Command(executable, nativeErrorFlag, title, message)
		if err = cmd.Start(); err == nil {
			_ = cmd.Wait()
			return
		}
	}

	// A child could not be started. Keep the GUI-only contract by showing the
	// same GTK 4 alert in this process instead of falling back to text.
	showFatalErrorInProcess(title, message)
}

func showFatalErrorInProcess(title, message string) {
	cTitle := C.CString(title)
	cMessage := C.CString(message)
	defer C.free(unsafe.Pointer(cTitle))
	defer C.free(unsafe.Pointer(cMessage))
	C.nativeui_show_fatal_error(cTitle, cMessage)
}
