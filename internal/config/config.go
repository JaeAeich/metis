package config

import (
	"strings"

	"github.com/spf13/viper"
)

// Cfg is the global configuration object.
var Cfg *Config

// Config holds the application's configuration.
type Config struct {
	Metel MetelConfig `mapstructure:"METEL"`
	Log   LogConfig   `mapstructure:"LOG"`
	API   APIConfig   `mapstructure:"API"`
}

// LoadCommonConfig loads the common configuration.
func LoadCommonConfig() error {
	viper.SetEnvPrefix("METIS")
	viper.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))
	viper.AutomaticEnv()

	viper.SetDefault("ENVIRONMENT", "dev")
	viper.SetDefault("LOG_LEVEL", "info")

	var config Config
	if err := viper.Unmarshal(&config); err != nil {
		return err
	}

	Cfg = &config
	return nil
}

// LoadMetelConfig loads the Metel configuration.
func LoadMetelConfig() error {
	return LoadCommonConfig()
}

// LoadAPIConfig loads the API configuration.
func LoadAPIConfig() error {
	if err := LoadCommonConfig(); err != nil {
		return err
	}
	viper.SetDefault("API.SERVER.PORT", 8080)

	// Swagger
	viper.SetDefault("API.SWAGGER.BASE_PATH", "/")
	viper.SetDefault("API.SWAGGER.PATH", "/ui")
	viper.SetDefault("API.SWAGGER.TITLE", "Metis API")

	return viper.Unmarshal(&Cfg)
}
