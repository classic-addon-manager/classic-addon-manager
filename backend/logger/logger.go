package logger

import (
	"fmt"
	"log"
	"os"
	"path/filepath"
	"time"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

var logger *zap.SugaredLogger

func GetLogPath() string {
	path, err := os.UserConfigDir()
	if err != nil {
		log.Fatalf("Error getting user config dir: %v", err)
	}
	return filepath.Join(path, "ClassicAddonManager", "app.log")
}

func buildLogger(outputPaths []string) *zap.SugaredLogger {
	if err := os.MkdirAll(filepath.Dir(GetLogPath()), 0700); err != nil {
		log.Fatalf("Error creating log directory: %v", err)
	}

	c := zap.NewProductionConfig()
	c.EncoderConfig.EncodeTime = zapcore.TimeEncoderOfLayout(time.RFC3339)
	c.OutputPaths = outputPaths
	l, err := c.Build(zap.AddCaller(), zap.AddCallerSkip(1))
	if err != nil {
		log.Fatalf("Error initializing logger: %v", err)
	}

	return l.Sugar()
}

func getLogger() *zap.SugaredLogger {
	if logger == nil {
		initLogger()
	}
	return logger
}

func Sync() {
	if logger != nil {
		_ = logger.Sync()
	}
}

func Info(msg string) {
	fmt.Println(msg)      // stdout
	getLogger().Info(msg) // app.log
}

func Warn(msg string) {
	fmt.Println(msg)      // stdout
	getLogger().Warn(msg) // app.log
}

func Error(msg string, err error) {
	fmt.Println(msg, err)       // stdout
	getLogger().Error(msg, err) // app.log
}

func Fatal(msg string, err error) {
	fmt.Println(msg, err)       // stdout
	getLogger().Fatal(msg, err) // app.log
}
