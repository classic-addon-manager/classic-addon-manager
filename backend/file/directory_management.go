package file

import (
	"os"
)

func RemoveDir(path string) (bool, error) {
	err := os.RemoveAll(path)
	if err != nil {
		return false, err
	}
	return true, nil
}
