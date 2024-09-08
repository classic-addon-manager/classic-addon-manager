package file

import (
	"os"
	"syscall"
	"time"
)

// GetModifiedTime - This function only functions on Windows
func GetModifiedTime(path string) (time.Time, error) {
	info, err := os.Stat(path)
	if err != nil {
		return time.Time{}, err
	}

	if stat, ok := info.Sys().(*syscall.Win32FileAttributeData); ok {
		return time.Unix(0, stat.LastWriteTime.Nanoseconds()), nil
	}

	return time.Time{}, nil
}
