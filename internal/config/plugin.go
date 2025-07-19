package config

// PluginConfig holds the configuration for a single plugin.
type PluginConfig struct {
	WorkflowEngineVersion string `mapstructure:"workflow_engine_version"`
	WorkflowType          string `mapstructure:"workflow_type"`
	WorkflowTypeVersion   string `mapstructure:"workflow_type_version"`
	PluginURL             string `mapstructure:"plugin_url"`
}
