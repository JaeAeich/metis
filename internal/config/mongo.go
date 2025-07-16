package config

// MongoConfig holds the mongodb configuration.
type MongoConfig struct {
	Host               string `mapstructure:"HOST"`
	Username           string `mapstructure:"USERNAME"`
	Password           string `mapstructure:"PASSWORD"`
	Database           string `mapstructure:"DATABASE"`
	WorkflowCollection string `mapstructure:"WORKFLOW_COLLECTION"`
	Port               int    `mapstructure:"PORT"`
}
