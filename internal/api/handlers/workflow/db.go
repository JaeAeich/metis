// Package run has business logic for creating and viewing runs
package run

import (
	"context"
	"time"

	"go.mongodb.org/mongo-driver/bson"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/schema"
)

// InsertRunLog inserts a new run log into the database using the schema structure.
func InsertRunLog(runID string, runRequest *api.RunRequest) error {
	// Create initial workflow document with basic run log
	workflowDoc := schema.NewWorkflowCollection(runID)
	workflowDoc.Workflow.RunLog = &api.RunLog{
		RunId: &runID,
		State: func() *api.State {
			s := api.QUEUED
			return &s
		}(),
		Request: runRequest,
		RunLog: func() *api.Log {
			startTime := time.Now().Format(time.RFC3339)
			return &api.Log{
				Name:      &runID,
				StartTime: &startTime,
			}
		}(),
	}

	_, err := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection).InsertOne(context.Background(), workflowDoc)
	return err
}

// UpdateWorkflowStatus updates the workflow status, and sets start time if transitioning to RUNNING.
func UpdateWorkflowStatus(runID string, status api.State, startTime *string) error {
	filter := bson.M{"run_id": runID}
	updateFields := bson.M{
		"workflow.run_log.state": status,
		"updated_at":             time.Now(),
	}

	// If transitioning to RUNNING and startTime is provided, update the start time
	if status == api.RUNNING && startTime != nil {
		updateFields["workflow.run_log.run_log.start_time"] = *startTime
	}

	update := bson.M{"$set": updateFields}

	_, err := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection).UpdateOne(
		context.Background(),
		filter,
		update,
	)
	return err
}

// UpdateWorkflowWithError updates workflow with error state, stderr message, and system logs.
func UpdateWorkflowWithError(runID string, errorMessage string, systemLogs string) error {
	filter := bson.M{"run_id": runID}
	endTime := time.Now().Format(time.RFC3339)

	update := bson.M{
		"$set": bson.M{
			"workflow.run_log.state":               api.SYSTEMERROR,
			"workflow.run_log.run_log.stderr":      errorMessage,
			"workflow.run_log.run_log.system_logs": systemLogs,
			"workflow.run_log.run_log.end_time":    endTime,
			"updated_at":                           time.Now(),
		},
	}

	_, err := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection).UpdateOne(
		context.Background(),
		filter,
		update,
	)
	return err
}

// UpdateWorkflowComplete updates a workflow document with completed execution data.
func UpdateWorkflowComplete(workflowDoc *schema.WorkflowCollection) error {
	workflowDoc.UpdatedAt = time.Now()

	_, err := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection).ReplaceOne(
		context.Background(),
		map[string]interface{}{"run_id": workflowDoc.RunID},
		workflowDoc,
	)
	return err
}
