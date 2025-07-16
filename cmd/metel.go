package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/config"
)

func handleMetelCmd() {
	metelCmd := flag.NewFlagSet("metel", flag.ExitOnError)
	workflowURL := metelCmd.String("workflow_url", "", "URL to the workflow")
	workflowType := metelCmd.String("workflow_type", "", "Type of the workflow")
	workflowTypeVersion := metelCmd.String("workflow_type_version", "", "Version of the workflow type")
	workflowEngine := metelCmd.String("workflow_engine", "", "Workflow engine to use")
	workflowEngineVersion := metelCmd.String("workflow_engine_version", "", "Version of the workflow engine")
	workflowParams := metelCmd.String("workflow_params", "", "JSON string of workflow parameters")
	workflowEngineParameters := metelCmd.String("workflow_engine_parameters", "", "JSON string of workflow engine parameters")
	tags := metelCmd.String("tags", "", "JSON string of tags")

	err := metelCmd.Parse(os.Args[2:])
	if err != nil {
		fmt.Println("error parsing metel command", err)
		os.Exit(1)
	}

	runRequest := api.RunRequest{
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

	fmt.Printf("running workflow: %+v\n", runRequest)

	files, err := os.ReadDir(config.Cfg.K8s.PVCMountPath)
	if err != nil {
		fmt.Println("error reading transfer directory", err)
		// This might not be a fatal error, maybe no attachments were sent.
	}

	fmt.Println("workflow attachments:")
	for _, file := range files {
		fmt.Printf("  - %s\n", file.Name())
	}
}
