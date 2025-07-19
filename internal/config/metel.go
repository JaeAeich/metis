package config

// StagingConfig holds the configuration for the remote staging area.
type StagingConfig struct {
	Parameters map[string]string `mapstructure:"PARAMETERS"`
	Type       string            `mapstructure:"TYPE"`
	Bucket     string            `mapstructure:"BUCKET"`
	Prefix     string            `mapstructure:"PREFIX"`
	URL        string            `mapstructure:"URL"`
}

// MetelConfig holds the configuration for the Metel service.
type MetelConfig struct {
	Staging StagingConfig `mapstructure:"STAGING"`
}
