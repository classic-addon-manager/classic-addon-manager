package file

import (
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
)

func MoveDir(src string, dest string) error {
	// Remove destination if it exists
	if _, err := os.Stat(dest); err == nil {
		if err := os.RemoveAll(dest); err != nil {
			return fmt.Errorf("failed to remove existing destination: %w", err)
		}
	}

	// Try atomic rename first
	if err := os.Rename(src, dest); err == nil {
		return nil
	}

	// If rename fails, do a manual copy and delete (handles cross-device moves)
	if err := CopyDir(src, dest); err != nil {
		return fmt.Errorf("failed to copy directory: %w", err)
	}

	// Remove original after successful copy
	if err := os.RemoveAll(src); err != nil {
		return fmt.Errorf("failed to remove source directory: %w", err)
	}

	return nil
}

func CopyDir(src string, dest string) error {
	entries, err := os.ReadDir(src)
	if err != nil {
		return err
	}

	if err := os.MkdirAll(dest, os.ModePerm); err != nil {
		return err
	}

	for _, entry := range entries {
		srcPath := filepath.Join(src, entry.Name())
		destPath := filepath.Join(dest, entry.Name())

		if entry.IsDir() {
			if err := CopyDir(srcPath, destPath); err != nil {
				return err
			}
		} else {
			data, err := os.ReadFile(srcPath)
			if err != nil {
				return err
			}
			if err := os.WriteFile(destPath, data, fs.FileMode(0644)); err != nil {
				return err
			}
		}
	}

	return nil
}
