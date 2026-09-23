//go:build windows

package services

import (
	"path/filepath"
	"strings"

	"golang.org/x/sys/windows"
)

func resolveAddonDirectory(path string) (string, error) {
	name, err := windows.UTF16PtrFromString(path)
	if err != nil {
		return "", err
	}
	handle, err := windows.CreateFile(name, 0, windows.FILE_SHARE_READ|windows.FILE_SHARE_WRITE|windows.FILE_SHARE_DELETE,
		nil, windows.OPEN_EXISTING, windows.FILE_FLAG_BACKUP_SEMANTICS, 0)
	if err != nil {
		return "", err
	}
	defer windows.CloseHandle(handle)

	size, err := windows.GetFinalPathNameByHandle(handle, nil, 0, 0)
	if err != nil {
		return "", err
	}
	buffer := make([]uint16, size+1)
	var length uint32
	for {
		length, err = windows.GetFinalPathNameByHandle(handle, &buffer[0], uint32(len(buffer)), 0)
		if err != nil {
			return "", err
		}
		if length < uint32(len(buffer)) {
			break
		}
		buffer = make([]uint16, length+1)
	}
	resolved := windows.UTF16ToString(buffer[:length])
	if strings.HasPrefix(resolved, `\\?\UNC\`) {
		resolved = `\\` + strings.TrimPrefix(resolved, `\\?\UNC\`)
	} else {
		resolved = strings.TrimPrefix(resolved, `\\?\`)
	}
	return filepath.Clean(resolved), nil
}
