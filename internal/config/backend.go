package config

// BackendType is the type of the backend.
type BackendType string

const (
	// BackendTypeLocal define local backend, ie the wf will be directly executed on the pod.
	BackendTypeLocal BackendType = "local"
	// BackendTypeTes define TES execution backend.
	BackendTypeTes BackendType = "tes"
)

// TesConfig holds configuration for TES backend.
type TesConfig struct {
	URL string `mapstructure:"URL"`
}

// LocalConfig holds configuration for Local backend.
type LocalConfig struct{}

// ExecutionBackendConfig holds the configuration for the execution backend.
type ExecutionBackendConfig struct {
	TesConfig   *TesConfig   `mapstructure:"TES_CONFIG"`
	LocalConfig *LocalConfig `mapstructure:"LOCAL_CONFIG"`
	Type        BackendType  `mapstructure:"TYPE"`
}
