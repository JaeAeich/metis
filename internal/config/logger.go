package config

// LogConfig holds the logging configuration.
type LogConfig struct {
	Level  string `mapstructure:"LEVEL"`
	Format string `mapstructure:"FORMAT"`
}
