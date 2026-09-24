package logger

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime/debug"
	"slices"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"gopkg.in/natefinch/lumberjack.v2"
)

const (
	maxSizeMB        = 10
	maxBackups       = 3
	headlessMaxBytes = 1 << 20
)

var headless atomic.Bool

var get = sync.OnceValue(func() *zap.SugaredLogger {
	l, _ := openLogger(headless.Load())
	return l
})

// UseHeadlessLog routes logging to check-updates.log instead of the rotating
// app.log, so a headless run can safely overlap the GUI.
func UseHeadlessLog() {
	headless.Store(true)
}

func LogPath() (string, error) {
	dir, err := os.UserConfigDir()
	if err != nil {
		return "", fmt.Errorf("resolve user config dir: %w", err)
	}
	return filepath.Join(dir, "ClassicAddonManager", "app.log"), nil
}

func openLogger(headless bool) (*zap.SugaredLogger, io.Closer) {
	var console zapcore.WriteSyncer
	if !isProductionBuild() {
		console = zapcore.Lock(os.Stdout)
	}

	appLogPath, err := LogPath()
	if err == nil {
		var l *zap.SugaredLogger
		var closer io.Closer
		if headless {
			path := filepath.Join(filepath.Dir(appLogPath), "check-updates.log")
			l, closer, err = newAppendLogger(path, headlessMaxBytes, console)
		} else {
			l, closer, err = newFileLogger(appLogPath, maxSizeMB, maxBackups, console)
		}
		if err == nil {
			return l, closer
		}
	}
	l := newStderrLogger()
	l.Warnw("file logging unavailable", "error", err)
	return l, nil
}

func newFileLogger(path string, maxSizeMB, maxBackups int, console zapcore.WriteSyncer) (*zap.SugaredLogger, io.Closer, error) {
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return nil, nil, fmt.Errorf("create log directory: %w", err)
	}
	w := &lumberjack.Logger{
		Filename:   path,
		MaxSize:    maxSizeMB,
		MaxBackups: maxBackups,
	}
	// lumberjack opens the file lazily, a zero-length write forces the open now
	// so failures reach openLogger's fallback instead of being dropped later.
	if _, err := w.Write(nil); err != nil {
		return nil, nil, fmt.Errorf("open log file: %w", err)
	}
	return buildLogger(zapcore.AddSync(w), console), w, nil
}

func newAppendLogger(path string, maxBytes int64, console zapcore.WriteSyncer) (*zap.SugaredLogger, io.Closer, error) {
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return nil, nil, fmt.Errorf("create log directory: %w", err)
	}
	if info, err := os.Stat(path); err == nil && info.Size() > maxBytes {
		// Best effort: another headless run may hold the file on Windows, we just keep appending.
		_ = os.Rename(path, path+".1")
	}
	f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0600)
	if err != nil {
		return nil, nil, fmt.Errorf("open log file: %w", err)
	}
	return buildLogger(zapcore.AddSync(f), console), f, nil
}

func buildLogger(file, console zapcore.WriteSyncer) *zap.SugaredLogger {
	core := newCore(file)
	if console != nil {
		cenc := encoderConfig()
		cenc.EncodeLevel = zapcore.CapitalLevelEncoder
		consoleCore := zapcore.NewCore(zapcore.NewConsoleEncoder(cenc), console, zapcore.InfoLevel)
		core = zapcore.NewTee(core, consoleCore)
	}
	return zap.New(core, loggerOptions()...).Sugar()
}

func isProductionBuild() bool {
	info, ok := debug.ReadBuildInfo()
	return ok && hasBuildTag(info.Settings, "production")
}

func hasBuildTag(settings []debug.BuildSetting, tag string) bool {
	for _, s := range settings {
		if s.Key == "-tags" {
			return slices.Contains(strings.Split(s.Value, ","), tag)
		}
	}
	return false
}

func newStderrLogger() *zap.SugaredLogger {
	return zap.New(newCore(zapcore.Lock(os.Stderr)), loggerOptions()...).Sugar()
}

func encoderConfig() zapcore.EncoderConfig {
	enc := zap.NewProductionEncoderConfig()
	enc.EncodeTime = zapcore.TimeEncoderOfLayout(time.RFC3339)
	return enc
}

func newCore(ws zapcore.WriteSyncer) zapcore.Core {
	return zapcore.NewCore(zapcore.NewJSONEncoder(encoderConfig()), ws, zapcore.InfoLevel)
}

func loggerOptions() []zap.Option {
	return []zap.Option{
		zap.AddCaller(),
		zap.AddCallerSkip(1),
		zap.ErrorOutput(zapcore.Lock(os.Stderr)),
	}
}

func Sync() {
	_ = get().Sync()
}

func Info(msg string) {
	get().Info(msg)
}

func Warn(msg string) {
	get().Warn(msg)
}

func Error(msg string, errs ...error) {
	logError(get(), msg, errs...)
}

func logError(l *zap.SugaredLogger, msg string, errs ...error) {
	// Account for this extra frame so the caller annotation matches Info/Warn.
	l = l.WithOptions(zap.AddCallerSkip(1))
	if err := errors.Join(errs...); err != nil {
		l.Errorw(msg, "error", err)
	} else {
		l.Error(msg)
	}
}
