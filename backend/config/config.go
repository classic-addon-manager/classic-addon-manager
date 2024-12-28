package config

import (
	"errors"
	"fmt"
	"github.com/spf13/viper"
	"github.com/sqweek/dialog"
	"os"
	"path/filepath"
)

var aacPath = "C:\\AAClassic\\Documents"

func setDefaults() {
	// GENERAL
	viper.SetDefault("general.aacpath", aacPath)
}

func LoadConfig() error {
	err := getOrCreateConfig()
	if err != nil {
		return err
	}

	path, err := detectAACPath()
	if err != nil {
		dialog.Message("Could not detect ArcheAge Classic installation: %s", err.Error()).Title("Classic Addon Manager Error").Error()
		return err
	}

	aacPath = path + "\\Documents"

	fmt.Println("Detected ArcheAge Classic installation at", aacPath)

	// Workaround for early testers
	SetString("general.aacpath", aacPath)

	return nil
}

func detectAACPath() (string, error) {
	// Find all instances of C:\AAClassic* and get the one with the highest number
	matches, err := filepath.Glob("C:\\AAClassic*")
	if err != nil {
		return "", err
	}

	if len(matches) == 0 {
		return "", errors.New("could not find any ArcheAge Classic installations")
	}

	var highestVersion string
	for _, match := range matches {
		if highestVersion == "" {
			highestVersion = match
			continue
		}
		if match > highestVersion {
			highestVersion = match
		}
	}

	return highestVersion, nil
}

//goland:noinspection GoTypeAssertionOnErrors
func getOrCreateConfig() error {
	configDir, err := os.UserConfigDir()
	if err != nil {
		return errors.New("could not get user config directory")
	}

	managerDir := filepath.Join(configDir, "ClassicAddonManager")
	_, err = os.Stat(managerDir)
	// Ensure the config directory exists
	if err != nil {
		if os.IsNotExist(err) {
			err := os.Mkdir(managerDir, 0700)
			if err != nil {
				return errors.New("could not create config directory")
			}
			fmt.Println("Created config directory")
		}
	}

	// Ensure the config file exists
	_, err = os.Stat(filepath.Join(managerDir, "config.toml"))
	if err != nil {
		if os.IsNotExist(err) {
			_, err = os.Create(filepath.Join(managerDir, "config.toml"))
			if err != nil {
				return errors.New("could not create config file")
			}
		}
	}

	viper.SetConfigName("config.toml")
	viper.AddConfigPath(managerDir)
	viper.SetConfigFile(filepath.Join(managerDir, "config.toml"))
	setDefaults()

	err = viper.ReadInConfig()
	if err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); !ok {
			// Error reading config file but not because it's missing
			return fmt.Errorf("could not read config: %w", err)
		}
	}

	err = viper.WriteConfigAs(filepath.Join(managerDir, "config.toml"))
	if err != nil {
		return fmt.Errorf("could not write config file: %w", err)
	}

	return nil
}

func SaveConfig() error {
	err := viper.WriteConfig()
	if err != nil {
		return fmt.Errorf("could not save config: %w", err)
	}
	return nil
}

func getUserDocumentsFolder() (string, error) {
	userProfile := os.Getenv("USERPROFILE")
	if userProfile == "" {
		return "", errors.New("could not get user profile")
	}
	documentsFolder := filepath.Join(userProfile, "Documents")

	return documentsFolder, nil
}

func GetCacheDir() string {
	c, err := os.UserCacheDir()
	if err != nil {
		return ""
	}
	cacheDir := filepath.Join(c, "ClassicAddonManager")
	_, err = os.Stat(cacheDir)
	if err != nil {
		if os.IsNotExist(err) {
			err := os.Mkdir(cacheDir, 0700)
			if err != nil {
				fmt.Println("Could not create cache directory")
				return ""
			}
			fmt.Println("Created cache directory")
		}
	}
	return cacheDir
}

func GetDataDir() string {
	configDir, err := os.UserConfigDir()
	if err != nil {
		dialog.Message("could not get user config directory: %s", err.Error()).Title("Classic Addon Manager Error").Error()
		return ""
	}

	managerDir := filepath.Join(configDir, "ClassicAddonManager")
	_, err = os.Stat(managerDir)
	// Ensure the config directory exists
	if err != nil {
		if os.IsNotExist(err) {
			err := os.Mkdir(managerDir, 0700)
			if err != nil {
				dialog.Message("could not create config directory: %s", err.Error()).Title("Classic Addon Manager Error").Error()
				return ""
			}
			fmt.Println("Created config directory")
		}
	}

	return managerDir
}

func GetAACDir() string {
	return aacPath
}

func GetAddonDir() string {
	return filepath.Join(GetAACDir(), "Addon")
}

func GetBool(option string) bool {
	return viper.GetBool(option)
}

func SetBool(option string, value bool) {
	viper.Set(option, value)
	_ = SaveConfig()
}

func GetString(option string) string {
	return viper.GetString(option)
}

func SetString(option string, value string) {
	viper.Set(option, value)
	_ = SaveConfig()
}
