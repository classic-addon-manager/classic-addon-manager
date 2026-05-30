//go:build windows

package logger

import (
	"net/url"
	"os"

	"go.uber.org/zap"
)

func newWinFileSink(u *url.URL) (zap.Sink, error) {
	// Remove the leading slash from Windows file URLs like /C:/Users/...
	return os.OpenFile(u.Path[1:], os.O_WRONLY|os.O_APPEND|os.O_CREATE, 0644)
}

func initLogger() {
	_ = zap.RegisterSink("winfile", newWinFileSink)
	logger = buildLogger([]string{"winfile:///" + GetLogPath()})
}
