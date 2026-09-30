//go:build !linux

package updater

func selfUpdateAppImage(updateURL, checksum string) error {
	return ErrSelfUpdateUnsupported
}
