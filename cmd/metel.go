package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/types/known/structpb"

	api "github.com/jaeaeich/metis/internal/api/generated"
	workflowDB "github.com/jaeaeich/metis/internal/api/handlers/workflow"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/errors"
	"github.com/jaeaeich/metis/internal/logger"
	"github.com/jaeaeich/metis/internal/metel/proto"
	"github.com/jaeaeich/metis/internal/metel/staging"
	"github.com/jaeaeich/metis/internal/metel/workflow"
	"github.com/jaeaeich/metis/internal/metel/workflow/download"
	"github.com/jaeaeich/metis/internal/schema"
)

func handleMetelCmd() {
	runRequest, runID, err := parseParams()

	// Capture start time at the very beginning
	startTime := time.Now().Format(time.RFC3339)

	if err != nil {
		logger.L.Error("error parsing parameters", "error", err)
		// Update workflow with error state (runID might be empty, so we skip DB update here)
		os.Exit(1)
	}

	plugin, err := getPlugin(runRequest)
	if err != nil {
		logger.L.Error("error getting plugin", "error", err)
		errorMsg := err.Error()
		systemLogs := "Failed to find suitable plugin for workflow type: " + runRequest.WorkflowType
		if updateErr := workflowDB.UpdateWorkflowWithError(runID, errorMsg, systemLogs); updateErr != nil {
			logger.L.Error("failed to update workflow with error", "run_id", runID, "error", updateErr)
		}
		os.Exit(1)
	}

	primaryDescriptor, err := downloadWorkflow(runRequest)
	if err != nil {
		logger.L.Error("error downloading workflow", "error", err)
		errorMsg := err.Error()
		systemLogs := "Failed to download workflow from URL: " + runRequest.WorkflowUrl
		if updateErr := workflowDB.UpdateWorkflowWithError(runID, errorMsg, systemLogs); updateErr != nil {
			logger.L.Error("failed to update workflow with error", "run_id", runID, "error", updateErr)
		}
		os.Exit(1)
	}

	executionSpec, err := getExecutionSpec(plugin, runRequest, primaryDescriptor, runID)
	if err != nil {
		logger.L.Error("could not get execution spec", "error", err)
		errorMsg := err.Error()
		systemLogs := "Failed to get execution spec from plugin: " + plugin.PluginURL
		if updateErr := workflowDB.UpdateWorkflowWithError(runID, errorMsg, systemLogs); updateErr != nil {
			logger.L.Error("failed to update workflow with error", "run_id", runID, "error", updateErr)
		}
		os.Exit(1)
	}

	if launchErr := workflow.LaunchJob(executionSpec, runID); launchErr != nil {
		logger.L.Error("failed to launch job", "error", launchErr)
		errorMsg := launchErr.Error()
		systemLogs := "Failed to launch Kubernetes job for run ID: " + runID
		if updateErr := workflowDB.UpdateWorkflowWithError(runID, errorMsg, systemLogs); updateErr != nil {
			logger.L.Error("failed to update workflow with error", "run_id", runID, "error", updateErr)
		}
		os.Exit(1)
	}

	// Update workflow status to RUNNING after job is launched
	if updateErr := workflowDB.UpdateWorkflowStatus(runID, api.RUNNING, &startTime); updateErr != nil {
		logger.L.Error("failed to update workflow status to RUNNING", "run_id", runID, "error", updateErr)
	} else {
		logger.L.Info("workflow status updated", "run_id", runID, "status", "RUNNING", "start_time", startTime)
	}

	result, err := workflow.WatchJob(context.Background(), runID)
	if err != nil {
		logger.L.Error("failed to watch job", "error", err)
		errorMsg := err.Error()
		systemLogs := "Failed to watch Kubernetes job status for run ID: " + runID
		if updateErr := workflowDB.UpdateWorkflowWithError(runID, errorMsg, systemLogs); updateErr != nil {
			logger.L.Error("failed to update workflow with error", "run_id", runID, "error", updateErr)
		}
		os.Exit(1)
	}

	switch result.Status {
	case workflow.JobSucceeded:
		if stageErr := stageLocalData(executionSpec, runID); stageErr != nil {
			logger.L.Error("failed to stage local data", "error", stageErr)
		}
	case workflow.JobFailedCommand:
		logger.L.Error("command failed", "error", result.Message)
	case workflow.JobFailedSystem:
		logger.L.Error("system failed", "error", result.Message)
	}

	endTime := time.Now().Format(time.RFC3339)

	parsedRunLog, err := parseExecution(plugin, runID, result.Logs, result)
	if err != nil {
		logger.L.Error("failed to parse execution", "error", err)
		errorMsg := err.Error()
		systemLogs := "Failed to parse execution results from plugin: " + plugin.PluginURL
		if updateErr := workflowDB.UpdateWorkflowWithError(runID, errorMsg, systemLogs); updateErr != nil {
			logger.L.Error("failed to update workflow with error", "run_id", runID, "error", updateErr)
		}
		os.Exit(1)
	}

	outputs := make(map[string]interface{})
	if parsedRunLog.Outputs != nil {
		for k, v := range parsedRunLog.Outputs {
			outputs[k] = v.AsInterface()
		}
	}

	// Convert task logs to TaskLog format for database schema
	taskLogs := make([]api.TaskLog, len(parsedRunLog.TaskLogs))
	for i, log := range parsedRunLog.TaskLogs {
		taskLogs[i] = api.TaskLog{
			Name:       &log.Name,
			Cmd:        &log.Cmd,
			StartTime:  &log.StartTime,
			EndTime:    &log.EndTime,
			ExitCode:   &log.ExitCode,
			Stderr:     &log.Stderr,
			Stdout:     &log.Stdout,
			SystemLogs: &log.SystemLogs,
		}
	}

	// Determine final state based on job result
	var finalState api.State
	switch result.Status {
	case workflow.JobSucceeded:
		finalState = api.COMPLETE
	default:
		finalState = api.COMPLETE // Even failed jobs are marked as complete
	}

	// Create RunLog for database schema
	runLog := &api.RunLog{
		RunId: &runID,
		State: &finalState,
		RunLog: &api.Log{
			Name:       &parsedRunLog.RunLog.Name,
			Cmd:        &executionSpec.Command,
			StartTime:  &startTime,
			EndTime:    &endTime,
			ExitCode:   &parsedRunLog.RunLog.ExitCode,
			Stdout:     &parsedRunLog.RunLog.Stdout,
			Stderr:     &parsedRunLog.RunLog.Stderr,
			SystemLogs: &parsedRunLog.RunLog.SystemLogs,
		},
		Request: runRequest,
		Outputs: &outputs,
	}

	// Create workflow document using database schema
	workflowDoc := schema.NewWorkflowCollection(runID)
	workflowDoc.Workflow.RunLog = runLog
	workflowDoc.Workflow.Tasks = taskLogs

	// Update completed workflow data in database
	if err := workflowDB.UpdateWorkflowComplete(workflowDoc); err != nil {
		logger.L.Error("failed to update workflow in database", "run_id", runID, "error", err)
	} else {
		logger.L.Info("workflow completed and saved to database", "run_id", runID, "status", finalState)
	}
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

func parseExecution(plugin *config.PluginConfig, runID, jobLogs string, result *workflow.JobResult) (*proto.WesRunLog, error) {
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

	ctx, cancel := context.WithTimeout(context.Background(), time.Second*10)
	defer cancel()

	var state proto.ParseState
	switch result.Status {
	case workflow.JobSucceeded:
		state = proto.ParseState_SUCCESS
	case workflow.JobFailedCommand:
		state = proto.ParseState_FAILURE
	case workflow.JobFailedSystem:
		state = proto.ParseState_FAILURE
	default:
		state = proto.ParseState_UNKNOWN_STATE
	}

	fmt.Printf("stagingURL: %s\n", stagingURL)
	fmt.Printf("endpointURL: %s\n", config.Cfg.Metel.Staging.URL)
	fmt.Printf("parameters: %v\n", config.Cfg.Metel.Staging.Parameters)
	fmt.Printf("state: %v\n", state)

	return c.ParseExecution(ctx, &proto.ParseExecutionRequest{
		JobLogs: jobLogs,
		StagingInfo: &proto.StagingInfo{
			StagingUrl:  stagingURL,
			EndpointUrl: config.Cfg.Metel.Staging.URL,
			Parameters:  config.Cfg.Metel.Staging.Parameters,
		},
		State: state,
	})
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

func stageLocalData(spec *proto.ExecutionSpec, runID string) error {
	if len(spec.OutputsToStage) == 0 {
		return nil
	}
	provider, err := staging.GetProvider()
	if err != nil {
		return fmt.Errorf("failed to get staging provider: %w", err)
	}

	stagingURL, err := provider.GetURL(runID)
	if err != nil {
		return fmt.Errorf("failed to get remote staging area: %w", err)
	}
	stagingInfo := &proto.StagingInfo{
		StagingUrl:  stagingURL,
		EndpointUrl: config.Cfg.Metel.Staging.URL,
		Parameters:  config.Cfg.Metel.Staging.Parameters,
	}

	for _, p := range spec.OutputsToStage {
		logger.L.Info("outputdir", "path", p)
		localPath := path.Join(config.Cfg.K8s.PVCMountPath, p)
		remotePath := path.Join(config.Cfg.Metel.Staging.Prefix, runID, p)
		logger.L.Info("outputdir", "localPath", localPath, "remotePath", remotePath)

		stat, err := os.Stat(localPath)
		if os.IsNotExist(err) {
			logger.L.Warn("output not found, skipping", "path", localPath)
			continue
		}
		if err != nil {
			return fmt.Errorf("failed to stat output %s: %w", p, err)
		}
		if stat.IsDir() {
			if err := provider.UploadDir(localPath, remotePath, stagingInfo); err != nil {
				return fmt.Errorf("failed to upload directory %s: %w", p, err)
			}
		} else {
			if err := provider.UploadFile(localPath, remotePath, stagingInfo); err != nil {
				return fmt.Errorf("failed to upload file %s: %w", p, err)
			}
		}
	}
	return nil
}
