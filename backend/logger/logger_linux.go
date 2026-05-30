//go:build linux

package logger

func initLogger() {
	logger = buildLogger([]string{GetLogPath()})
}
