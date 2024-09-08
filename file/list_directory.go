package file

import (
	"os"
)

// ListDirectories returns a list of directories in the specified path
func ListDirectories(path string) ([]string, error) {
	var directories []string

	// Open the directory
	dir, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer dir.Close()

	// Read directory entries
	entries, err := dir.Readdir(-1)
	if err != nil {
		return nil, err
	}

	// Filter entries to include only directories
	for _, entry := range entries {
		if entry.IsDir() {
			directories = append(directories, entry.Name())
		}
	}

	return directories, nil
}

func RemoveDir(path string) (bool, error) {
	err := os.RemoveAll(path)
	if err != nil {
		return false, err
	}
	return true, nil
}
