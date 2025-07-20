package config

// K8sConfig holds the Kubernetes configuration.
type K8sConfig struct {
	Namespace              string `mapstructure:"NAMESPACE"`
	ImagePullPolicy        string `mapstructure:"IMAGE_PULL_POLICY"`
	PVCAccessMode          string `mapstructure:"PVC_ACCESS_MODE"`
	PVCStorageClass        string `mapstructure:"PVC_STORAGE_CLASS"`
	CommonPVCVolumeName    string `mapstructure:"COMMON_PVC_VOLUME_NAME"`
	RestartPolicy          string `mapstructure:"RESTART_POLICY"`
	MetelPrefix            string `mapstructure:"METEL_PREFIX"`
	WePrefix               string `mapstructure:"WE_PREFIX"`
	ConfigPath             string `mapstructure:"CONFIG_PATH"`
	PVCPrefix              string `mapstructure:"PVC_PREFIX"`
	DefaultPVCSize         string `mapstructure:"DEFAULT_PVC_SIZE"`
	PVCMountPath           string `mapstructure:"PVC_MOUNT_PATH"`
	ImageName              string `mapstructure:"IMAGE_NAME"`
	PluginConfigMapName    string `mapstructure:"PLUGIN_CONFIG_MAP_NAME"`
	ServiceAccountName     string `mapstructure:"SERVICE_ACCOUNT_NAME"`
	JobTTL                 int    `mapstructure:"JOB_TTL"`
	SecurityContextEnabled bool   `mapstructure:"SECURITY_CONTEXT_ENABLED"`
}
