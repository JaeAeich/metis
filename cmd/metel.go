package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"time"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/errors"
	"github.com/jaeaeich/metis/internal/logger"
	"github.com/jaeaeich/metis/internal/metel/proto"
	"github.com/jaeaeich/metis/internal/metel/staging"
	"github.com/jaeaeich/metis/internal/metel/workflow"
	"github.com/jaeaeich/metis/internal/metel/workflow/download"
)

// TODO: Update WorkflowDB if err.
func handleMetelCmd() {
	runRequest, runID, err := parseParams()
	if err != nil {
		logger.L.Error("error parsing parameters", "error", err)
		os.Exit(1)
	}

	wesRequestBytes, marshalErr := json.MarshalIndent(runRequest, "", "  ")
	if marshalErr != nil {
		logger.L.Error("error marshaling WES request", "error", marshalErr)
		os.Exit(1)
	}
	fmt.Println("--- WES Request ---")
	fmt.Println(string(wesRequestBytes))
	fmt.Println("--------------------")

	plugin, err := getPlugin(runRequest)
	if err != nil {
		logger.L.Error("error getting plugin", "error", err)
		os.Exit(1)
	}

	fmt.Println("--- Plugin Selected ---")
	fmt.Printf("Plugin URL: %s\n", plugin.PluginURL)
	fmt.Printf("Workflow Type: %s\n", plugin.WorkflowType)
	fmt.Printf("Workflow Type Version: %s\n", plugin.WorkflowTypeVersion)
	fmt.Printf("Workflow Engine Version: %s\n", plugin.WorkflowEngineVersion)
	fmt.Println("-----------------------")
	primaryDescriptor, err := downloadWorkflow(runRequest)
	if err != nil {
		logger.L.Error("error downloading workflow", "error", err)
		os.Exit(1)
	}

	fmt.Println("--- Files in PVC ---")
	files, err := os.ReadDir(config.Cfg.K8s.PVCMountPath)
	if err != nil {
		logger.L.Error("error reading PVC directory", "path", config.Cfg.K8s.PVCMountPath, "error", err)
	} else {
		for _, file := range files {
			fmt.Println(file.Name())
		}
	}
	fmt.Println("--------------------")
}

func parseParams() (*api.RunRequest, string, error) {
	metelCmd := flag.NewFlagSet("metel", flag.ExitOnError)
	workflowURL := metelCmd.String("workflow_url", "", "URL to the workflow")
	workflowType := metelCmd.String("workflow_type", "", "Type of the workflow")
	workflowTypeVersion := metelCmd.String("workflow_type_version", "", "Version of the workflow type")
	workflowEngine := metelCmd.String("workflow_engine", "", "Workflow engine to use")
	workflowEngineVersion := metelCmd.String("workflow_engine_version", "", "Version of the workflow engine")
	workflowParams := metelCmd.String("workflow_params", "", "JSON string of workflow parameters")
	workflowEngineParameters := metelCmd.String("workflow_engine_parameters", "", "JSON string of workflow engine parameters")
	tags := metelCmd.String("tags", "", "JSON string of tags")
	runID := metelCmd.String("run_id", "", "The ID of the workflow run")

	if err := metelCmd.Parse(os.Args[2:]); err != nil {
		return nil, "", fmt.Errorf("error parsing metel command: %w", err)
	}

	runRequest := &api.RunRequest{
		WorkflowUrl:         *workflowURL,
		WorkflowType:        *workflowType,
		WorkflowTypeVersion: *workflowTypeVersion,
	}

	if *workflowEngine != "" {
		runRequest.WorkflowEngine = workflowEngine
	}
	if *workflowEngineVersion != "" {
		runRequest.WorkflowEngineVersion = workflowEngineVersion
	}
	if *workflowParams != "" {
		var params map[string]interface{}
		if errJSON := json.Unmarshal([]byte(*workflowParams), &params); errJSON == nil {
			runRequest.WorkflowParams = &params
		}
	}
	if *workflowEngineParameters != "" {
		var params map[string]string
		if errJSON := json.Unmarshal([]byte(*workflowEngineParameters), &params); errJSON == nil {
			runRequest.WorkflowEngineParameters = &params
		}
	}
	if *tags != "" {
		var t map[string]string
		if errJSON := json.Unmarshal([]byte(*tags), &t); errJSON == nil {
			runRequest.Tags = &t
		}
	}

	return runRequest, *runID, nil
}

func getPlugin(runRequest *api.RunRequest) (*config.PluginConfig, error) {
	for _, plugin := range config.Cfg.Plugins {
		if plugin.WorkflowType == runRequest.WorkflowType && plugin.WorkflowEngineVersion == *runRequest.WorkflowEngineVersion {
			return &plugin, nil
		}
	}
	return nil, errors.ErrNoSuitablePlugin
}

func downloadWorkflow(runRequest *api.RunRequest) (string, error) {
	downloader, err := download.GetDownloader(runRequest.WorkflowUrl)
	if err != nil {
		return "", fmt.Errorf("failed to get downloader: %w", err)
	}
	primaryDescriptor, err := downloader.Download(runRequest.WorkflowUrl, config.Cfg.K8s.PVCMountPath)
	if err != nil {
		return "", fmt.Errorf("failed to download workflow: %w", err)
	}
	return primaryDescriptor, nil
}

func getExecutionSpec(plugin *config.PluginConfig, runRequest *api.RunRequest, primaryDescriptor, runID string) (*proto.ExecutionSpec, error) {
	provider, err := staging.GetProvider()
	if err != nil {
		return nil, fmt.Errorf("failed to get staging provider: %w", err)
	}
	stagingURL, err := provider.GetURL(runID)
	if err != nil {
		return nil, fmt.Errorf("failed to get remote staging area: %w", err)
	}

	conn, err := grpc.NewClient(plugin.PluginURL, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, fmt.Errorf("did not connect: %w", err)
	}
	defer func() {
		if closeErr := conn.Close(); closeErr != nil {
			logger.L.Error("failed to close connection", "error", closeErr)
		}
	}()
	c := proto.NewPluginExecutionClient(conn)

	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()

	workflowParamsStruct, err := structpb.NewStruct(*runRequest.WorkflowParams)
	if err != nil {
		return nil, fmt.Errorf("failed to convert workflow params to structpb: %w", err)
	}

	return c.GetExecutionSpec(ctx, &proto.GetExecutionSpecRequest{
		WesRequest: &proto.WesRequest{
			WorkflowUrl:              runRequest.WorkflowUrl,
			WorkflowType:             runRequest.WorkflowType,
			WorkflowTypeVersion:      runRequest.WorkflowTypeVersion,
			WorkflowParams:           workflowParamsStruct.GetFields(),
			WorkflowEngine:           *runRequest.WorkflowEngine,
			WorkflowEngineVersion:    *runRequest.WorkflowEngineVersion,
			WorkflowEngineParameters: *runRequest.WorkflowEngineParameters,
			Tags:                     *runRequest.Tags,
		},
		StagingInfo: &proto.StagingInfo{
			StagingUrl:  stagingURL,
			EndpointUrl: config.Cfg.Metel.Staging.URL,
			Parameters:  config.Cfg.Metel.Staging.Parameters,
		},
		PrimaryDescriptor: primaryDescriptor,
	})
}
