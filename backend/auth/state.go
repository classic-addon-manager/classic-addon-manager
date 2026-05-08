package auth

import "sync"

var (
	mu    sync.RWMutex
	token string
)

func GetToken() string {
	mu.RLock()
	defer mu.RUnlock()
	return token
}

func SetToken(t string) {
	mu.Lock()
	defer mu.Unlock()
	token = t
}

func ClearToken() {
	mu.Lock()
	defer mu.Unlock()
	token = ""
}
