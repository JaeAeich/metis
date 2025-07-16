package config

// K8sConfig holds the Kubernetes configuration.
type K8sConfig struct {
	ImagePullPolicy        string `mapstructure:"IMAGE_PULL_POLICY"`
	MetelPrefix            string `mapstructure:"METEL_PREFIX"`
	PVCAccessMode          string `mapstructure:"PVC_ACCESS_MODE"`
	PVCStorageClass        string `mapstructure:"PVC_STORAGE_CLASS"`
	CommonPVCVolumeName    string `mapstructure:"COMMON_PVC_VOLUME_NAME"`
	RestartPolicy          string `mapstructure:"RESTART_POLICY"`
	DefaultPVCSize         string `mapstructure:"DEFAULT_PVC_SIZE"`
	PVCMountPath           string `mapstructure:"PVC_MOUNT_PATH"`
	Namespace              string `mapstructure:"NAMESPACE"`
	PVCPrefix              string `mapstructure:"PVC_PREFIX"`
	ConfigPath             string `mapstructure:"CONFIG_PATH"`
	ImageName              string `mapstructure:"IMAGE_NAME"`
	JobTTL                 int    `mapstructure:"JOB_TTL"`
	SecurityContextEnabled bool   `mapstructure:"SECURITY_CONTEXT_ENABLED"`
}
