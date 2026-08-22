//go:build windows

package nativeui

import "golang.org/x/sys/windows"

// HandleNativeErrorArgs also keeps direct invocations of the hidden mode
// isolated from normal application startup on Windows.
func HandleNativeErrorArgs(args []string) bool {
	title, message, ok := parseNativeErrorArgs(args)
	if !ok {
		return false
	}

	ShowFatalError(title, message)
	return true
}

// ShowFatalError displays a blocking system-modal Win32 error message box.
func ShowFatalError(title, message string) {
	messagePtr, err := windows.UTF16PtrFromString(message)
	if err != nil {
		return
	}
	titlePtr, err := windows.UTF16PtrFromString(title)
	if err != nil {
		return
	}

	_, _ = windows.MessageBox(
		0,
		messagePtr,
		titlePtr,
		windows.MB_OK|windows.MB_ICONERROR|windows.MB_SYSTEMMODAL,
	)
}
