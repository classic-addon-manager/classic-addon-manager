package file

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
)

func MoveFile(src string, dest string) error {
	srcInfo, err := os.Stat(src)
	if err != nil {
		return fmt.Errorf("source file does not exist: %s", src)
	}
	if !srcInfo.Mode().IsRegular() {
		return fmt.Errorf("cannot move non-regular file: %s", src)
	}

	destDir := filepath.Dir(dest)
	dirInfo, err := os.Stat(destDir)
	if err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("destination directory does not exist: %s", destDir)
		}
		return fmt.Errorf("error getting info for destination directory: %s", destDir)
	}
	if !dirInfo.IsDir() {
		return fmt.Errorf("destination is not a directory: %s", destDir)
	}

	srcFile, err := os.Open(src)
	if err != nil {
		return fmt.Errorf("error opening source file: %s", src)
	}
	defer srcFile.Close()

	destFile, err := os.OpenFile(dest, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, srcInfo.Mode().Perm())
	if err != nil {
		return fmt.Errorf("error opening destination file: %s", dest)
	}

	_, copyErr := io.Copy(destFile, srcFile)
	closeErr := destFile.Close()

	// Cleanup if copy fails
	if copyErr != nil || closeErr != nil {
		_ = os.Remove(dest)
		if copyErr != nil {
			return fmt.Errorf("error copying file: %s", copyErr)
		}
		return fmt.Errorf("error closing destination file: %s", closeErr)
	}

	// Preserve modified time
	if err := os.Chtimes(dest, srcInfo.ModTime(), srcInfo.ModTime()); err != nil {
		return fmt.Errorf("error setting modified time: %s", err)
	}

	return nil
}
