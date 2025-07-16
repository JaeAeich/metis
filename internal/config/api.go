// Package config provides the configuration for the Metis application.
package config

// ServerConfig holds the server configuration.
type ServerConfig struct {
	Host     string `mapstructure:"HOST"`
	BasePath string `mapstructure:"BASE_PATH"`
	Port     int    `mapstructure:"PORT"`
}

// SwaggerConfig holds the Swagger configuration.
type SwaggerConfig struct {
	Path  string `mapstructure:"PATH"`
	Title string `mapstructure:"TITLE"`
}

// APIConfig holds the configuration for the API server.
type APIConfig struct {
	Swagger SwaggerConfig `mapstructure:"SWAGGER"`
	Server  ServerConfig  `mapstructure:"SERVER"`
}
