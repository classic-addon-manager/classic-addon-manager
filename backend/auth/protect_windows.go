//go:build windows

package auth

import (
	"encoding/base64"
	"fmt"
	"syscall"
	"unsafe"
)

var (
	crypt32dll             = syscall.NewLazyDLL("crypt32.dll")
	procCryptProtectData   = crypt32dll.NewProc("CryptProtectData")
	procCryptUnprotectData = crypt32dll.NewProc("CryptUnprotectData")
	kernel32dll            = syscall.NewLazyDLL("kernel32.dll")
	procLocalFree          = kernel32dll.NewProc("LocalFree")
)

type dataBlob struct {
	cbData uint32
	pbData *byte
}

func dpapiProtect(data []byte) ([]byte, error) {
	if len(data) == 0 {
		return nil, fmt.Errorf("empty data")
	}
	in := dataBlob{
		cbData: uint32(len(data)),
		pbData: &data[0],
	}
	var out dataBlob

	r, _, err := procCryptProtectData.Call(
		uintptr(unsafe.Pointer(&in)),
		0,
		0,
		0,
		0,
		0,
		uintptr(unsafe.Pointer(&out)),
	)
	if r == 0 {
		return nil, fmt.Errorf("CryptProtectData failed: %w", err)
	}
	defer localFree(out.pbData)

	outSlice := unsafe.Slice(out.pbData, out.cbData)
	result := make([]byte, out.cbData)
	copy(result, outSlice)
	return result, nil
}

func dpapiUnprotect(data []byte) ([]byte, error) {
	if len(data) == 0 {
		return nil, fmt.Errorf("empty data")
	}
	in := dataBlob{
		cbData: uint32(len(data)),
		pbData: &data[0],
	}
	var out dataBlob

	r, _, err := procCryptUnprotectData.Call(
		uintptr(unsafe.Pointer(&in)),
		0,
		0,
		0,
		0,
		0,
		uintptr(unsafe.Pointer(&out)),
	)
	if r == 0 {
		return nil, fmt.Errorf("CryptUnprotectData failed: %w", err)
	}
	defer localFree(out.pbData)

	outSlice := unsafe.Slice(out.pbData, out.cbData)
	result := make([]byte, out.cbData)
	copy(result, outSlice)
	return result, nil
}

func localFree(ptr *byte) {
	procLocalFree.Call(uintptr(unsafe.Pointer(ptr)))
}

func protectToken(plainToken string) (string, string, error) {
	encrypted, err := dpapiProtect([]byte(plainToken))
	if err != nil {
		return "", "", fmt.Errorf("dpapi encrypt failed: %w", err)
	}
	encoded := base64.StdEncoding.EncodeToString(encrypted)
	return encoded, "dpapi", nil
}

func unprotectToken(encoding, token string) (string, error) {
	if encoding == "dpapi" {
		decoded, err := base64.StdEncoding.DecodeString(token)
		if err != nil {
			return "", fmt.Errorf("error decoding base64: %w", err)
		}
		decrypted, err := dpapiUnprotect(decoded)
		if err != nil {
			return "", fmt.Errorf("dpapi decrypt failed: %w", err)
		}
		return string(decrypted), nil
	}
	if encoding == "plain" {
		return token, nil
	}
	return "", fmt.Errorf("unknown encoding: %s", encoding)
}

func afterWriteHook(path string) error {
	return nil
}
