package logger

import (
	"ClassicAddonManager/config"
	"fmt"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"log"
	"net/url"
	"os"
	"path/filepath"
	"time"
)

var logger *zap.SugaredLogger

func GetLogPath() string {
	return filepath.Join(config.GetDataDir(), "app.log")
}

func newWinFileSink(u *url.URL) (zap.Sink, error) {
	// Remove leading slash left by url.Parse()
	return os.OpenFile(u.Path[1:], os.O_WRONLY|os.O_APPEND|os.O_CREATE, 0644)
}

func initLogger() {
	_ = zap.RegisterSink("winfile", newWinFileSink)
	c := zap.NewProductionConfig()
	c.EncoderConfig.EncodeTime = zapcore.TimeEncoderOfLayout(time.RFC3339)
	c.OutputPaths = []string{"winfile:///" + GetLogPath()}
	l, err := c.Build(zap.AddCaller(), zap.AddCallerSkip(1))

	if err != nil {
		log.Fatalf("Error initializing logger: %v", err)
	}

	sugar := l.Sugar()
	defer sugar.Sync()

	logger = sugar
}

func getLogger() *zap.SugaredLogger {
	if logger == nil {
		initLogger()
	}
	return logger
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
