// Package workflow provides the workflow execution logic for metel.
package workflow

import (
	"context"
	"fmt"
	"strings"

	batchv1 "k8s.io/api/batch/v1"
	v1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
	"github.com/jaeaeich/metis/internal/metel/proto"
)

// LaunchJob creates and launches a Kubernetes job for a workflow run.
func LaunchJob(spec *proto.ExecutionSpec, runID string) error {
	// Prepare and create configmaps for root and project files.
	rootCMName, rootVolumeMounts, err := prepareAndCreateConfigMap(runID, "root", spec.RootMountFiles, func(path string) string {
		return path
	})
	if err != nil {
		return err
	}

	projectCMName, projectVolumeMounts, err := prepareAndCreateConfigMap(runID, "project", spec.ProjectMountFiles, func(path string) string {
		return fmt.Sprintf("%s/%s", strings.TrimRight(config.Cfg.K8s.PVCMountPath, "/"), strings.TrimLeft(path, "/"))
	})
	if err != nil {
		return err
	}

	// Aggregate volumes and volume mounts.
	volumes := buildVolumes(runID, rootCMName, projectCMName)
	volumeMounts := buildVolumeMounts(rootVolumeMounts, projectVolumeMounts)

	// Build and create the Kubernetes job.
	job := buildJob(runID, spec, volumes, volumeMounts)
	createdJob, err := clients.K8s.BatchV1().Jobs(config.Cfg.K8s.Namespace).Create(context.Background(), job, metav1.CreateOptions{})
	if err != nil {
		return fmt.Errorf("failed to create job: %w", err)
	}

	// Set owner references for garbage collection.
	var cmNames []string
	if rootCMName != "" {
		cmNames = append(cmNames, rootCMName)
	}
	if projectCMName != "" {
		cmNames = append(cmNames, projectCMName)
	}
	if len(cmNames) > 0 {
		updateOwnerReferences(createdJob, cmNames)
	}

	return nil
}

func prepareAndCreateConfigMap(runID, name string, files map[string]string, mountPathFunc func(string) string) (string, []v1.VolumeMount, error) {
	if len(files) == 0 {
		return "", nil, nil
	}

	data := make(map[string]string)
	volumeMounts := make([]v1.VolumeMount, 0, len(files))
	volumeName := fmt.Sprintf("%s-files", name)

	for path, content := range files {
		key := strings.Trim(path, "/")
		data[key] = content
		volumeMounts = append(volumeMounts, v1.VolumeMount{
			Name:      volumeName,
			MountPath: mountPathFunc(path),
			SubPath:   key,
		})
	}

	cmName, err := createConfigMap(runID, name, data)
	if err != nil {
		return "", nil, fmt.Errorf("failed to create %s configmap: %w", name, err)
	}

	return cmName, volumeMounts, nil
}

func buildVolumes(runID, rootCMName, projectCMName string) []v1.Volume {
	volumes := []v1.Volume{
		{
			Name: config.Cfg.K8s.CommonPVCVolumeName,
			VolumeSource: v1.VolumeSource{
				PersistentVolumeClaim: &v1.PersistentVolumeClaimVolumeSource{
					ClaimName: fmt.Sprintf("%s-%s", config.Cfg.K8s.PVCPrefix, runID),
				},
			},
		},
	}
	if rootCMName != "" {
		volumes = append(volumes, v1.Volume{
			Name: "root-files",
			VolumeSource: v1.VolumeSource{
				ConfigMap: &v1.ConfigMapVolumeSource{
					LocalObjectReference: v1.LocalObjectReference{
						Name: rootCMName,
					},
				},
			},
		})
	}
	if projectCMName != "" {
		volumes = append(volumes, v1.Volume{
			Name: "project-files",
			VolumeSource: v1.VolumeSource{
				ConfigMap: &v1.ConfigMapVolumeSource{
					LocalObjectReference: v1.LocalObjectReference{
						Name: projectCMName,
					},
				},
			},
		})
	}
	return volumes
}

func buildVolumeMounts(rootVolumeMounts, projectVolumeMounts []v1.VolumeMount) []v1.VolumeMount {
	volumeMounts := []v1.VolumeMount{
		{
			Name:      config.Cfg.K8s.CommonPVCVolumeName,
			MountPath: config.Cfg.K8s.PVCMountPath,
		},
	}
	volumeMounts = append(volumeMounts, rootVolumeMounts...)
	volumeMounts = append(volumeMounts, projectVolumeMounts...)
	return volumeMounts
}

func buildJob(runID string, spec *proto.ExecutionSpec, volumes []v1.Volume, volumeMounts []v1.VolumeMount) *batchv1.Job {
	return &batchv1.Job{
		ObjectMeta: metav1.ObjectMeta{
			Name:      fmt.Sprintf("%s-%s", config.Cfg.K8s.WePrefix, runID),
			Namespace: config.Cfg.K8s.Namespace,
			Labels: map[string]string{
				"app":             "metis",
				"metis/run-id":    runID,
				"metis/component": config.Cfg.K8s.WePrefix,
			},
		},
		Spec: batchv1.JobSpec{
			TTLSecondsAfterFinished: func() *int32 {
				//nolint:gosec // G115: We are confident that this conversion is safe.
				ttl := int32(config.Cfg.K8s.JobTTL)
				return &ttl
			}(),
			BackoffLimit: func() *int32 {
				backoffLimit := int32(0)
				return &backoffLimit
			}(),
			Template: v1.PodTemplateSpec{
				Spec: v1.PodSpec{
					RestartPolicy: v1.RestartPolicy(config.Cfg.K8s.RestartPolicy),
					Containers: []v1.Container{
						{
							Name:         fmt.Sprintf("%s-%s", config.Cfg.K8s.WePrefix, runID),
							Image:        spec.Image,
							Command:      spec.Command,
							WorkingDir:   config.Cfg.K8s.PVCMountPath,
							Env:          toK8sEnv(spec.Environment),
							VolumeMounts: volumeMounts,
						},
					},
					Volumes:            volumes,
					ServiceAccountName: config.Cfg.K8s.ServiceAccountName,
				},
			},
		},
	}
}

func createConfigMap(runID, name string, data map[string]string) (string, error) {
	if len(data) == 0 {
		return "", nil
	}
	cmName := fmt.Sprintf("%s-%s-%s", config.Cfg.K8s.WePrefix, runID, name)
	cm := &v1.ConfigMap{
		ObjectMeta: metav1.ObjectMeta{
			Name:      cmName,
			Namespace: config.Cfg.K8s.Namespace,
		},
		Data: data,
	}

	_, err := clients.K8s.CoreV1().ConfigMaps(config.Cfg.K8s.Namespace).Create(context.Background(), cm, metav1.CreateOptions{})
	if err != nil {
		return "", err
	}
	return cmName, nil
}

func toK8sEnv(env map[string]string) []v1.EnvVar {
	envVars := make([]v1.EnvVar, 0, len(env))
	for k, v := range env {
		envVars = append(envVars, v1.EnvVar{Name: k, Value: v})
	}
	return envVars
}

func updateOwnerReferences(job *batchv1.Job, cmNames []string) {
	isController := true
	ownerRef := metav1.OwnerReference{
		APIVersion: "batch/v1",
		Kind:       "Job",
		Name:       job.Name,
		UID:        job.UID,
		Controller: &isController,
	}

	for _, cmName := range cmNames {
		cm, getErr := clients.K8s.CoreV1().ConfigMaps(config.Cfg.K8s.Namespace).Get(context.Background(), cmName, metav1.GetOptions{})
		if getErr != nil {
			logger.L.Error("failed to get configmap to update owner reference", "error", getErr, "configmap", cmName)
			continue
		}
		cm.OwnerReferences = append(cm.OwnerReferences, ownerRef)
		_, err := clients.K8s.CoreV1().ConfigMaps(config.Cfg.K8s.Namespace).Update(context.Background(), cm, metav1.UpdateOptions{})
		if err != nil {
			logger.L.Error("failed to update configmap with owner reference", "error", err, "configmap", cmName)
		}
	}
}
