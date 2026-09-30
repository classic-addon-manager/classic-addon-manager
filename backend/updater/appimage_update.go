package updater

import (
	"ClassicAddonManager/backend/config"
	"ClassicAddonManager/backend/util"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

func updateFailurePath() (string, error) {
	if selfUpdateOS != "linux" {
		return filepath.Join(updateTempDir(), updateFailureFileName), nil
	}
	cacheDir, err := config.GetCacheDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(cacheDir, updateFailureFileName), nil
}

// APPIMAGE identifies the outer image, os.Executable points inside its temporary mount.
// Resolve symlinks so updating an image does not replace a user's launcher symlink.
func appImageTarget() (string, error) {
	path := os.Getenv("APPIMAGE")
	if !filepath.IsAbs(path) {
		return "", ErrSelfUpdateUnsupported
	}
	path, err := filepath.EvalSymlinks(path)
	if err != nil {
		return "", fmt.Errorf("locating AppImage: %w", err)
	}
	info, err := os.Stat(path)
	if err != nil {
		return "", err
	}
	if !info.Mode().IsRegular() || (runtime.GOOS == "linux" && info.Mode().Perm()&0111 == 0) {
		return "", ErrSelfUpdateUnsupported
	}
	if err := validateAppImage(path); err != nil {
		return "", err
	}
	// Replacement needs a writable directory, not write access to the running inode.
	probe, err := os.MkdirTemp(filepath.Dir(path), ".classic-addon-manager-update-*")
	if err != nil {
		return "", fmt.Errorf("the AppImage directory is not writable: %w", err)
	}
	if err := os.Remove(probe); err != nil {
		return "", err
	}
	return path, nil
}

func validateAppImage(path string) error {
	f, err := os.Open(path)
	if err != nil {
		return err
	}
	defer f.Close()
	var header [11]byte
	_, err = io.ReadFull(f, header[:])
	if err != nil || string(header[:4]) != "\x7fELF" || string(header[8:11]) != "AI\x02" {
		return fmt.Errorf("the update must be a type 2 AppImage")
	}
	return nil
}

type appImageUpdate struct {
	TargetPath string
	SourcePath string
	StagingDir string
}

// Stage on the target filesystem so the helper's rename cannot become a cross-device copy.
// Every failure before the helper starts leaves the installed image untouched.
func prepareAppImageUpdate(updateURL, checksum string) (appImageUpdate, error) {
	target, err := appImageTarget()
	if err != nil {
		return appImageUpdate{}, err
	}
	checksum = strings.TrimSpace(checksum)
	digest, err := hex.DecodeString(checksum)
	if err != nil || len(digest) != 32 {
		return appImageUpdate{}, ErrUpdateVerificationFailed
	}
	info, err := os.Stat(target)
	if err != nil {
		return appImageUpdate{}, err
	}
	staging, err := os.MkdirTemp(filepath.Dir(target), ".classic-addon-manager-update-*")
	if err != nil {
		return appImageUpdate{}, err
	}
	ready := false
	defer func() {
		if !ready {
			os.RemoveAll(staging)
		}
	}()
	source := filepath.Join(staging, "update.AppImage")
	if err := util.DownloadFile(updateURL, source); err != nil {
		return appImageUpdate{}, err
	}
	if err := verifyUpdateChecksum(source, checksum); err != nil {
		return appImageUpdate{}, err
	}
	if err := validateAppImage(source); err != nil {
		return appImageUpdate{}, err
	}
	f, err := os.OpenFile(source, os.O_RDWR, 0)
	if err != nil {
		return appImageUpdate{}, err
	}
	if err := f.Chmod(info.Mode().Perm()); err != nil {
		f.Close()
		return appImageUpdate{}, err
	}
	syncErr := f.Sync()
	closeErr := f.Close()
	if syncErr != nil {
		return appImageUpdate{}, syncErr
	}
	if closeErr != nil {
		return appImageUpdate{}, closeErr
	}
	ready = true
	return appImageUpdate{TargetPath: target, SourcePath: source, StagingDir: staging}, nil
}

// Pass paths as positional arguments, never interpolated shell source. The helper waits
// for the old IPC server to disappear, then atomically replaces the image without a backup.
const appImageUpdateScript = `#!/bin/sh
parent=$1
target=$2
source=$3
staging=$4
failure=$5
while kill -0 "$parent" 2>/dev/null; do
    sleep 0.1
done
if ! mv -f -- "$source" "$target"; then
    printf 'The update could not replace "%s". The verified download remains at "%s".\n' "$target" "$source" > "$failure"
    "$target"
    exit 1
fi
rm -f -- "$staging/update.sh"
rmdir -- "$staging"
"$target"
status=$?
if [ "$status" -ne 0 ]; then
    printf 'The updated AppImage "%s" exited with status %s. Please download a working version manually, no backup was retained.\n' "$target" "$status" > "$failure"
fi
exit "$status"
`

// AppRun can add mounted libraries/tools to the environment. They must not leak
// into the helper or the next AppImage after the old mount has disappeared.
func appImageRelaunchEnvironment(environ []string, appDir string) []string {
	result := make([]string, 0, len(environ))
	for _, entry := range environ {
		key, value, found := strings.Cut(entry, "=")
		if !found || key == "APPIMAGE" || key == "APPDIR" || key == "ARGV0" || key == "OWD" {
			continue
		}
		if appDir != "" && strings.Contains(value, appDir) {
			parts := strings.Split(value, string(os.PathListSeparator))
			kept := parts[:0]
			for _, part := range parts {
				if part != appDir && !strings.HasPrefix(part, appDir+string(os.PathSeparator)) {
					kept = append(kept, part)
				}
			}
			if len(kept) == 0 {
				continue
			}
			value = strings.Join(kept, string(os.PathListSeparator))
		}
		result = append(result, key+"="+value)
	}
	return result
}
