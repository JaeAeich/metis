package config

import (
	"errors"
	"strings"

	"github.com/spf13/viper"
)

var (
	// Version is the application version, set at build time.
	Version string
	// GitCommit is the git commit hash, set at build time.
	GitCommit string
	// Cfg is the global configuration object.
	Cfg *Config
)

// PluginConfig holds the configuration for a single plugin.
type PluginConfig struct {
	WorkflowEngineVersion string `mapstructure:"workflow_engine_version"`
	WorkflowType          string `mapstructure:"workflow_type"`
	WorkflowTypeVersion   string `mapstructure:"workflow_type_version"`
	PluginURL             string `mapstructure:"plugin_url"`
}

// Config holds the application's configuration.
type Config struct {
	Metel   MetelConfig    `mapstructure:"METEL"`
	Log     LogConfig      `mapstructure:"LOG"`
	Mongo   MongoConfig    `mapstructure:"MONGO"`
	API     APIConfig      `mapstructure:"API"`
	K8s     K8sConfig      `mapstructure:"K8S"`
	Plugins []PluginConfig `mapstructure:"PLUGINS"`
}

// LoadCommonConfig loads the common configuration.
func LoadCommonConfig() error {
	viper.SetEnvPrefix("METIS")
	viper.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))
	viper.AutomaticEnv()

	// Load plugins config file
	viper.SetConfigName("plugins")
	viper.SetConfigType("yaml")
	viper.AddConfigPath("$HOME/.metis")
	if err := viper.ReadInConfig(); err != nil {
		var configFileNotFoundError viper.ConfigFileNotFoundError
		if errors.As(err, &configFileNotFoundError) {
			// Config file was found but another error was produced
			return err
		}
	}

	viper.SetDefault("ENVIRONMENT", "dev")
	viper.SetDefault("LOG.LEVEL", "info")
	viper.SetDefault("LOG.FORMAT", "text")
	viper.SetDefault("MONGO.HOST", "localhost")
	viper.SetDefault("MONGO.PORT", 27017)
	viper.SetDefault("MONGO.USERNAME", "")
	viper.SetDefault("MONGO.PASSWORD", "")
	viper.SetDefault("MONGO.DATABASE", "metis")
	viper.SetDefault("MONGO.WORKFLOW_COLLECTION", "workflows")

	viper.SetDefault("K8S.CONFIG_PATH", "")
	viper.SetDefault("K8S.NAMESPACE", "metis")
	viper.SetDefault("K8S.PVC_ACCESS_MODE", "")
	viper.SetDefault("K8S.PVC_STORAGE_CLASS", "")
	viper.SetDefault("K8S.COMMON_PVC_VOLUME_NAME", "workflow-pvc")
	viper.SetDefault("K8S.RESTART_POLICY", "Never")
	viper.SetDefault("K8S.IMAGE_PULL_POLICY", "IfNotPresent")
	viper.SetDefault("K8S.JOB_TTL", 300)
	viper.SetDefault("K8S.SECURITY_CONTEXT_ENABLED", false)
	viper.SetDefault("K8S.DEFAULT_PVC_SIZE", "100Mi")
	viper.SetDefault("K8S.PVC_PREFIX", "pvc")
	viper.SetDefault("K8S.METEL_PREFIX", "metel")
	viper.SetDefault("K8S.IMAGE_NAME", "jaeaeich/metis:latest")
	viper.SetDefault("K8S.PLUGIN_CONFIG_MAP_NAME", "metis-plugin-configmap")

	var config Config
	if err := viper.Unmarshal(&config); err != nil {
		return err
	}

	Cfg = &config
	Cfg.K8s.PVCMountPath = "/pvc"
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
	viper.SetDefault("API.SERVER.BASE_PATH", "/ga4gh/wes/v1")

	// Swagger
	viper.SetDefault("API.SWAGGER.PATH", "/ui")
	viper.SetDefault("API.SWAGGER.TITLE", "Metis API")

	return viper.Unmarshal(&Cfg)
}
