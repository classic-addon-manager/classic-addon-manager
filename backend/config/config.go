package config

import (
	"ClassicAddonManager/backend/logger"
	"errors"
	"fmt"
	"github.com/spf13/viper"
	"github.com/sqweek/dialog"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
)

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

	aacPath := path + "\\Documents"
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
	re := regexp.MustCompile(`^(C:\\AAClassic)(\d*)$`)

	for _, match := range matches {
		if highestVersion == "" {
			highestVersion = match
			continue
		}

		// Extract base path and version suffix
		currentMatch := re.FindStringSubmatch(match)
		highestMatch := re.FindStringSubmatch(highestVersion)

		// If paths don't match, fall back to lexicographical comparison
		if currentMatch == nil || highestMatch == nil {
			if match > highestVersion {
				highestVersion = match
			}
			continue
		}

		// Convert suffix to int for comparison
		currentNum := 0
		if currentMatch[2] != "" {
			currentNum, _ = strconv.Atoi(currentMatch[2])
		}

		highestNum := 0
		if highestMatch[2] != "" {
			highestNum, _ = strconv.Atoi(highestMatch[2])
		}

		// Compare numerically, but prefer explicit numbers over implicit zero
		if currentNum > highestNum || (currentNum == highestNum && currentMatch[2] != "" && highestMatch[2] == "") {
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
	path := viper.GetString("general.aacpath")
	if path == "" {
		dialog.Message("path to AAClassic is empty, this should not happen").Title("Classic Addon Manager Error").Error()
	}
	return path
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
	err := SaveConfig()
	if err != nil {
		dialog.Message("could not save config: %s", err.Error()).Title("Classic Addon Manager Error").Error()
		logger.Error("Could not save config: ", err)
	}
	logger.Info("Set config option: " + option + " to " + value)
}
