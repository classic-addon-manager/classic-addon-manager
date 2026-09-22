package file

import (
	"ClassicAddonManager/backend/logger"
	"archive/zip"
	"bufio"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

func FileExists(path string) bool {
	_, err := os.Stat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return false
		}
		logger.Error("Error checking if file exists:", err)
		return false
	}

	return true
}

func ReadLines(path string) ([]string, error) {
	var lines []string

	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}

	if err := scanner.Err(); err != nil {
		return nil, err
	}

	return lines, nil
}

func WriteLines(path string, lines []string) error {
	return WriteAtomic(path, []byte(strings.Join(lines, "\n")), 0644)
}

func WriteJSON(path string, data []byte) error {
	return WriteAtomic(path, data, 0644)
}

// createAtomicTemp avoids name collisions while letting the OS apply the process umask.
func createAtomicTemp(dir, base string, perm os.FileMode) (*os.File, error) {
	var suffix [8]byte
	for range 100 {
		if _, err := rand.Read(suffix[:]); err != nil {
			return nil, fmt.Errorf("failed to generate temp file name: %w", err)
		}
		tmpPath := filepath.Join(dir, "."+base+".tmp-"+hex.EncodeToString(suffix[:]))
		tmp, err := os.OpenFile(tmpPath, os.O_WRONLY|os.O_CREATE|os.O_EXCL, perm)
		if os.IsExist(err) {
			continue
		}
		if err != nil {
			return nil, fmt.Errorf("failed to create temp file: %w", err)
		}
		return tmp, nil
	}
	return nil, fmt.Errorf("failed to create temp file after repeated name collisions")
}

// WriteAtomic replaces a file only after its new contents are synced and closed.
func WriteAtomic(path string, data []byte, perm os.FileMode) (err error) {
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, os.ModePerm); err != nil {
		return fmt.Errorf("failed to create directories: %w", err)
	}

	// Preserve an existing mode; for new files, the process umask narrows perm.
	writePerm := perm
	preserveMode := false
	if info, statErr := os.Stat(path); statErr == nil {
		writePerm = info.Mode().Perm()
		preserveMode = true
	} else if !os.IsNotExist(statErr) {
		return fmt.Errorf("failed to inspect existing file: %w", statErr)
	}

	tmp, err := createAtomicTemp(dir, filepath.Base(path), writePerm)
	if err != nil {
		return err
	}
	tmpPath := tmp.Name()
	defer func() {
		_ = tmp.Close()
		_ = os.Remove(tmpPath)
	}()

	if preserveMode {
		if err := tmp.Chmod(writePerm); err != nil {
			return fmt.Errorf("failed to preserve file permissions: %w", err)
		}
	}
	if _, err := tmp.Write(data); err != nil {
		return fmt.Errorf("failed to write temp file: %w", err)
	}
	if err := tmp.Sync(); err != nil {
		return fmt.Errorf("failed to sync temp file: %w", err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("failed to close temp file: %w", err)
	}
	if err := os.Rename(tmpPath, path); err != nil {
		return fmt.Errorf("failed to replace file: %w", err)
	}
	return nil
}

func OpenDirectory(path string) error {
	var cmd *exec.Cmd

	switch runtime.GOOS {
	case "windows":
		cmd = exec.Command("explorer", path)
	case "linux":
		cmd = exec.Command("xdg-open", path)
	default:
		return fmt.Errorf("unsupported platform")
	}

	return cmd.Start()
}

func ValidateAddonZip(zipPath string) error {
	archive, err := zip.OpenReader(zipPath)
	if err != nil {
		return err
	}
	defer archive.Close()

	hasMainLua := false

	for _, f := range archive.File {
		// Iterate until we find main.lua
		if !strings.HasSuffix(f.Name, "main.lua") {
			continue
		}

		hasMainLua = true
		break
	}

	if !hasMainLua {
		return fmt.Errorf("invalid archive: main.lua not found in %s", zipPath)
	}

	return nil
}
