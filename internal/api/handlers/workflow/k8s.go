package run

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"mime/multipart"
	"strings"

	batchv1 "k8s.io/api/batch/v1"
	v1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
)

// CreateAttachmentConfigMaps creates configmaps for each workflow attachment.
func CreateAttachmentConfigMaps(runID string, attachments []*multipart.FileHeader) ([]string, []string, error) {
	attachmentConfigMaps := []string{}
	attachmentNames := make([]string, len(attachments))
	for i, attachment := range attachments {
		attachmentNames[i] = attachment.Filename

		file, openErr := attachment.Open()
		if openErr != nil {
			return nil, nil, fmt.Errorf("failed to open attachment %s: %w", attachment.Filename, openErr)
		}
		defer func() {
			if err := file.Close(); err != nil {
				logger.L.Error("failed to close attachment file %s: %v", attachment.Filename, err)
			}
		}()

		buf := new(bytes.Buffer)
		if _, readErr := buf.ReadFrom(file); readErr != nil {
			return nil, nil, fmt.Errorf("failed to read from attachment file %s: %w", attachment.Filename, readErr)
		}
		fileBytes := buf.Bytes()

		cmName := fmt.Sprintf("attachment-%s-%d", runID, i)
		configMap := &v1.ConfigMap{
			ObjectMeta: metav1.ObjectMeta{
				Name:      cmName,
				Namespace: config.Cfg.K8s.Namespace,
				Labels: map[string]string{
					"app":              "metis",
					"metis/run-id":     runID,
					"metis/attachment": attachment.Filename,
					"metis/component":  "attachment",
				},
			},
			BinaryData: map[string][]byte{
				attachment.Filename: fileBytes,
			},
		}

		_, createErr := clients.K8s.CoreV1().ConfigMaps(config.Cfg.K8s.Namespace).Create(context.Background(), configMap, metav1.CreateOptions{})
		if createErr != nil {
			return nil, nil, fmt.Errorf("failed to create configmap for attachment %s: %w", attachment.Filename, createErr)
		}
		attachmentConfigMaps = append(attachmentConfigMaps, cmName)
	}
	return attachmentConfigMaps, attachmentNames, nil
}

// CreatePVCForRun creates a PVC for a workflow run.
func CreatePVCForRun(runID string) (*v1.PersistentVolumeClaim, error) {
	pvc := &v1.PersistentVolumeClaim{
		ObjectMeta: metav1.ObjectMeta{
			Name:      fmt.Sprintf("%s-%s", config.Cfg.K8s.PVCPrefix, runID),
			Namespace: config.Cfg.K8s.Namespace,
			Labels: map[string]string{
				"app":             "metis",
				"metis/run-id":    runID,
				"metis/component": config.Cfg.K8s.PVCPrefix,
			},
		},
		Spec: v1.PersistentVolumeClaimSpec{
			AccessModes: []v1.PersistentVolumeAccessMode{v1.ReadWriteOnce},
			StorageClassName: func() *string {
				if config.Cfg.K8s.PVCStorageClass != "" {
					return &config.Cfg.K8s.PVCStorageClass
				}
				return nil
			}(),
			Resources: v1.VolumeResourceRequirements{
				Requests: v1.ResourceList{
					v1.ResourceStorage: resource.MustParse(config.Cfg.K8s.DefaultPVCSize),
				},
			},
		},
	}
	logger.L.Debug("creating pvc", "pvc", pvc)
	createdPvc, err := clients.K8s.CoreV1().PersistentVolumeClaims(config.Cfg.K8s.Namespace).Create(context.Background(), pvc, metav1.CreateOptions{})
	if err != nil {
		return nil, fmt.Errorf("failed to create pvc: %w", err)
	}
	return createdPvc, nil
}

// CreateMetelJob creates a job to run the workflow.
func CreateMetelJob(runID string, runRequest *api.RunRequest, pvcName string, attachmentConfigMaps []string) (*batchv1.Job, error) {
	args := buildMetelArgs(runRequest)

	metelJobName := fmt.Sprintf("%s-%s", config.Cfg.K8s.MetelPrefix, runID)
	job := &batchv1.Job{
		ObjectMeta: metav1.ObjectMeta{
			Name:      metelJobName,
			Namespace: config.Cfg.K8s.Namespace,
			Labels: map[string]string{
				"app":             "metis",
				"metis/run-id":    runID,
				"metis/component": config.Cfg.K8s.MetelPrefix,
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
					Volumes:        buildVolumes(pvcName, attachmentConfigMaps),
					InitContainers: buildInitContainers(attachmentConfigMaps),
					Containers: []v1.Container{
						{
							Name:            metelJobName,
							Image:           config.Cfg.K8s.ImageName,
							Args:            args,
							ImagePullPolicy: v1.PullPolicy(config.Cfg.K8s.ImagePullPolicy),
							VolumeMounts: []v1.VolumeMount{
								{
									Name:      config.Cfg.K8s.CommonPVCVolumeName,
									MountPath: config.Cfg.K8s.PVCMountPath,
								},
							},
						},
					},
					RestartPolicy: v1.RestartPolicy(config.Cfg.K8s.RestartPolicy),
				},
			},
		},
	}
	createdJob, err := clients.K8s.BatchV1().Jobs(config.Cfg.K8s.Namespace).Create(context.Background(), job, metav1.CreateOptions{})
	if err != nil {
		return nil, fmt.Errorf("failed to create job: %w", err)
	}
	return createdJob, nil
}

// UpdateOwnerReferences sets the job as the owner of the PVC and configmaps.
func UpdateOwnerReferences(job *batchv1.Job, pvcName string, attachmentConfigMaps []string) {
	isController := true
	ownerRef := metav1.OwnerReference{
		APIVersion: "batch/v1",
		Kind:       "Job",
		Name:       job.Name,
		UID:        job.UID,
		Controller: &isController,
	}

	pvc, err := clients.K8s.CoreV1().PersistentVolumeClaims(config.Cfg.K8s.Namespace).Get(context.Background(), pvcName, metav1.GetOptions{})
	if err != nil {
		logger.L.Error("failed to get pvc to update owner reference", "error", err, "pvc", pvcName)
	} else {
		pvc.OwnerReferences = append(pvc.OwnerReferences, ownerRef)
		_, err = clients.K8s.CoreV1().PersistentVolumeClaims(config.Cfg.K8s.Namespace).Update(context.Background(), pvc, metav1.UpdateOptions{})
		if err != nil {
			logger.L.Error("failed to update pvc with owner reference", "error", err, "pvc", pvcName)
		}
	}

	for _, cmName := range attachmentConfigMaps {
		cm, getErr := clients.K8s.CoreV1().ConfigMaps(config.Cfg.K8s.Namespace).Get(context.Background(), cmName, metav1.GetOptions{})
		if getErr != nil {
			logger.L.Error("failed to get configmap to update owner reference", "error", getErr, "configmap", cmName)
			continue
		}
		cm.OwnerReferences = append(cm.OwnerReferences, ownerRef)
		_, err = clients.K8s.CoreV1().ConfigMaps(config.Cfg.K8s.Namespace).Update(context.Background(), cm, metav1.UpdateOptions{})
		if err != nil {
			logger.L.Error("failed to update configmap with owner reference", "error", err, "configmap", cmName)
		}
	}
}

func buildMetelArgs(runRequest *api.RunRequest) []string {
	args := []string{"/metis", "metel"}
	if runRequest.WorkflowUrl != "" {
		args = append(args, "--workflow_url", runRequest.WorkflowUrl)
	}
	if runRequest.WorkflowType != "" {
		args = append(args, "--workflow_type", runRequest.WorkflowType)
	}
	if runRequest.WorkflowTypeVersion != "" {
		args = append(args, "--workflow_type_version", runRequest.WorkflowTypeVersion)
	}
	if runRequest.WorkflowEngine != nil && *runRequest.WorkflowEngine != "" {
		args = append(args, "--workflow_engine", *runRequest.WorkflowEngine)
	}
	if runRequest.WorkflowEngineVersion != nil && *runRequest.WorkflowEngineVersion != "" {
		args = append(args, "--workflow_engine_version", *runRequest.WorkflowEngineVersion)
	}
	if runRequest.WorkflowParams != nil {
		paramsBytes, marshalErr := json.Marshal(runRequest.WorkflowParams)
		if marshalErr == nil {
			args = append(args, "--workflow_params", string(paramsBytes))
		}
	}
	if runRequest.WorkflowEngineParameters != nil {
		paramsBytes, marshalErr := json.Marshal(runRequest.WorkflowEngineParameters)
		if marshalErr == nil {
			args = append(args, "--workflow_engine_parameters", string(paramsBytes))
		}
	}
	if runRequest.Tags != nil {
		paramsBytes, marshalErr := json.Marshal(runRequest.Tags)
		if marshalErr == nil {
			args = append(args, "--tags", string(paramsBytes))
		}
	}
	return args
}

func buildVolumes(pvcName string, attachmentConfigMaps []string) []v1.Volume {
	volumes := []v1.Volume{
		{
			Name: config.Cfg.K8s.CommonPVCVolumeName,
			VolumeSource: v1.VolumeSource{
				PersistentVolumeClaim: &v1.PersistentVolumeClaimVolumeSource{
					ClaimName: pvcName,
				},
			},
		},
	}
	for i, cmName := range attachmentConfigMaps {
		volumes = append(volumes, v1.Volume{
			Name: fmt.Sprintf("attachment-vol-%d", i),
			VolumeSource: v1.VolumeSource{
				ConfigMap: &v1.ConfigMapVolumeSource{
					LocalObjectReference: v1.LocalObjectReference{
						Name: cmName,
					},
				},
			},
		})
	}
	return volumes
}

func buildInitContainers(attachmentConfigMaps []string) []v1.Container {
	if len(attachmentConfigMaps) == 0 {
		return nil
	}

	volumeMounts := []v1.VolumeMount{
		{
			Name:      config.Cfg.K8s.CommonPVCVolumeName,
			MountPath: config.Cfg.K8s.PVCMountPath,
		},
	}

	copyCmds := make([]string, 0, len(attachmentConfigMaps))
	for i, cmName := range attachmentConfigMaps {
		volName := fmt.Sprintf("attachment-vol-%d", i)
		attachmentMountPath := fmt.Sprintf("/attachments-src/%s", cmName)
		volumeMounts = append(volumeMounts, v1.VolumeMount{
			Name:      volName,
			MountPath: attachmentMountPath,
			ReadOnly:  true,
		})
		copyCmds = append(copyCmds, fmt.Sprintf("cp -L %s/* %s/", attachmentMountPath, config.Cfg.K8s.PVCMountPath))
	}

	fullCommand := strings.Join(copyCmds, " && ")

	return []v1.Container{
		{
			Name:         "copy-attachments",
			Image:        "busybox",
			Command:      []string{"sh", "-c", fullCommand},
			VolumeMounts: volumeMounts,
		},
	}
}
