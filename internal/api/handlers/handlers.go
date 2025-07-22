// Package handlers provides the handlers for the Metis API.
package handlers

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
	"go.mongodb.org/mongo-driver/bson"
	"go.mongodb.org/mongo-driver/bson/primitive"
	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"

	api "github.com/jaeaeich/metis/internal/api/generated"
	run "github.com/jaeaeich/metis/internal/api/handlers/workflow"
	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
	"github.com/jaeaeich/metis/internal/schema"
)

// Metis is our handler struct.
type Metis struct{}

// Make sure we conform to the generated server interface.
var _ api.ServerInterface = (*Metis)(nil)

// ListRuns lists all the workflow runs.
func (m *Metis) ListRuns(c *fiber.Ctx, params api.ListRunsParams) error {
	collection := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection)

	// Set up pagination
	var limit int64 = 20 // default page size
	if params.PageSize != nil && *params.PageSize > 0 {
		limit = *params.PageSize
	}

	// Build query
	query := bson.M{}

	// If there's a next page token, filter documents after that ID
	if params.PageToken != nil && *params.PageToken != "" {
		objectID, err := primitive.ObjectIDFromHex(*params.PageToken)
		if err != nil {
			logger.L.Error("invalid page token", "page_token", *params.PageToken, "error", err)
			statusCode := int32(fiber.StatusBadRequest)
			errMsg := "Invalid page token"
			return c.Status(fiber.StatusBadRequest).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
		query["_id"] = bson.M{"$gt": objectID}
	}

	// Query workflows with cursor-based pagination
	findOptions := options.Find()
	findOptions.SetLimit(limit + 1)                     // Fetch one extra to check if there's a next page
	findOptions.SetSort(bson.D{{Key: "_id", Value: 1}}) // Sort by ObjectId for consistent ordering

	cursor, err := collection.Find(context.Background(), query, findOptions)
	if err != nil {
		logger.L.Error("failed to query workflows", "error", err)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := "Failed to query workflows"
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	// Close the cursor and check for error
	defer func() {
		if err := cursor.Close(context.Background()); err != nil {
			logger.L.Error("failed to close cursor", "error", err)
		}
	}()

	var workflows []schema.WorkflowCollection
	if err := cursor.All(context.Background(), &workflows); err != nil {
		logger.L.Error("failed to decode workflows", "error", err)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := "Failed to decode workflows"
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	// Determine if there are more pages and slice to page size
	var nextPageToken *string
	if int64(len(workflows)) > limit {
		// There's a next page, use the last document's ObjectId as the token
		lastDoc := workflows[limit]
		token := lastDoc.ID.Hex()
		nextPageToken = &token
		workflows = workflows[:limit] // Remove the extra document
	}

	// Convert to API format
	runs := make([]api.RunListResponse_Runs_Item, 0, len(workflows))
	for _, workflow := range workflows {
		state := api.UNKNOWN
		if workflow.Workflow.RunLog != nil && workflow.Workflow.RunLog.State != nil {
			state = *workflow.Workflow.RunLog.State
		}

		runStatus := api.RunStatus{
			RunId: workflow.RunID,
			State: &state,
		}

		var runItem api.RunListResponse_Runs_Item
		if err := runItem.FromRunStatus(runStatus); err != nil {
			logger.L.Error("failed to convert run status to run item", "error", err, "run_id", workflow.RunID)
			continue
		}
		runs = append(runs, runItem)
	}

	return c.JSON(api.RunListResponse{
		Runs:          &runs,
		NextPageToken: nextPageToken,
	})
}

// RunWorkflow runs a workflow.
func (m *Metis) RunWorkflow(c *fiber.Ctx) error {
	runID := uuid.New().String()
	logger.L.Info("starting workflow run", "run_id", runID)

	runRequest, err := run.ParseRunRequest(c)
	if err != nil {
		logger.L.Error("failed to parse multipart form", "error", err)
		statusCode := int32(fiber.StatusBadRequest)
		errMsg := "Failed to parse multipart form"
		return c.Status(fiber.StatusBadRequest).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}
	logger.L.Debug("parsed request", "run_request", runRequest)

	form, err := c.MultipartForm()
	if err != nil {
		logger.L.Error("failed to parse multipart form", "error", err)
		statusCode := int32(fiber.StatusBadRequest)
		errMsg := "Failed to parse multipart form"
		return c.Status(fiber.StatusBadRequest).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	var attachmentConfigMaps []string
	if attachments, ok := form.File["workflow_attachment"]; ok {
		var attachmentNames []string
		attachmentConfigMaps, attachmentNames, err = run.CreateAttachmentConfigMaps(runID, attachments)
		if err != nil {
			logger.L.Error("failed to create attachment config maps", "error", err)
			statusCode := int32(fiber.StatusInternalServerError)
			errMsg := "Failed to create attachment config maps"
			return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
		logger.L.Debug("received and saved workflow attachments", "files", attachmentNames)
	}

	pvc, err := run.CreatePVCForRun(runID)
	if err != nil {
		logger.L.Error("failed to create pvc", "error", err)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := fmt.Sprintf("failed to create pvc: %v", err)
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	job, err := run.CreateMetelJob(runID, runRequest, pvc.Name, attachmentConfigMaps)
	if err != nil {
		logger.L.Error("failed to create job", "error", err)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := fmt.Sprintf("failed to create job: %v", err)
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}
	logger.L.Debug("created job", "job_name", job.Name, "job_uid", job.UID)

	run.UpdateOwnerReferences(job, pvc.Name, attachmentConfigMaps)

	if err := run.InsertRunLog(runID, runRequest); err != nil {
		logger.L.Error("failed to insert run log", "error", err)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := fmt.Sprintf("failed to insert run log: %v", err)
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	logger.L.Info("successfully started workflow run", "run_id", runID)
	return c.Status(fiber.StatusOK).JSON(api.RunId{RunId: &runID})
}

// GetRunLog gets the log for a workflow run.
func (m *Metis) GetRunLog(c *fiber.Ctx, runID string) error {
	collection := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection)

	var workflow schema.WorkflowCollection
	err := collection.FindOne(context.Background(), bson.M{"run_id": runID}).Decode(&workflow)
	if err != nil {
		if errors.Is(err, mongo.ErrNoDocuments) {
			logger.L.Warn("workflow not found", "run_id", runID)
			statusCode := int32(fiber.StatusNotFound)
			errMsg := "Workflow not found"
			return c.Status(fiber.StatusNotFound).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
		logger.L.Error("failed to get workflow", "error", err, "run_id", runID)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := "Failed to get workflow"
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	if workflow.Workflow.RunLog == nil {
		logger.L.Warn("workflow has no run log", "run_id", runID)
		statusCode := int32(fiber.StatusNotFound)
		errMsg := "Run log not found"
		return c.Status(fiber.StatusNotFound).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	return c.JSON(*workflow.Workflow.RunLog)
}

// CancelRun cancels a workflow run.
func (m *Metis) CancelRun(c *fiber.Ctx, runID string) error {
	return c.JSON(api.RunId{RunId: &runID})
}

// GetRunStatus gets the status of a workflow run.
func (m *Metis) GetRunStatus(c *fiber.Ctx, runID string) error {
	collection := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection)

	var workflow schema.WorkflowCollection
	err := collection.FindOne(context.Background(), bson.M{"run_id": runID}).Decode(&workflow)
	if err != nil {
		if errors.Is(err, mongo.ErrNoDocuments) {
			logger.L.Warn("workflow not found", "run_id", runID)
			statusCode := int32(fiber.StatusNotFound)
			errMsg := "Workflow not found"
			return c.Status(fiber.StatusNotFound).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
		logger.L.Error("failed to get workflow", "error", err, "run_id", runID)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := "Failed to get workflow"
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	state := api.UNKNOWN
	if workflow.Workflow.RunLog != nil && workflow.Workflow.RunLog.State != nil {
		state = *workflow.Workflow.RunLog.State
	}

	return c.JSON(api.RunStatus{
		RunId: runID,
		State: &state,
	})
}

// ListTasks lists the tasks for a workflow run.
func (m *Metis) ListTasks(c *fiber.Ctx, runID string, params api.ListTasksParams) error {
	collection := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection)

	var workflow schema.WorkflowCollection
	err := collection.FindOne(context.Background(), bson.M{"run_id": runID}).Decode(&workflow)
	if err != nil {
		if errors.Is(err, mongo.ErrNoDocuments) {
			logger.L.Warn("workflow not found", "run_id", runID)
			statusCode := int32(fiber.StatusNotFound)
			errMsg := "Workflow not found"
			return c.Status(fiber.StatusNotFound).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
		logger.L.Error("failed to get workflow", "error", err, "run_id", runID)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := "Failed to get workflow"
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	tasks := workflow.Workflow.Tasks
	if tasks == nil {
		tasks = []api.TaskLog{}
	}

	// Apply cursor-based pagination for tasks
	limit := 20 // default page size
	if params.PageSize != nil && *params.PageSize > 0 {
		limit = int(*params.PageSize)
	}

	startIndex := 0
	if params.PageToken != nil && *params.PageToken != "" {
		// For in-memory pagination of tasks, we can use simple index-based approach
		if _, err := fmt.Sscanf(*params.PageToken, "task_%d", &startIndex); err != nil {
			logger.L.Error("invalid page token for tasks", "page_token", *params.PageToken, "error", err)
			statusCode := int32(fiber.StatusBadRequest)
			errMsg := "Invalid page token for tasks"
			return c.Status(fiber.StatusBadRequest).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
	}

	// Apply pagination
	end := startIndex + limit
	if end > len(tasks) {
		end = len(tasks)
	}

	if startIndex > len(tasks) {
		startIndex = len(tasks)
	}

	paginatedTasks := tasks[startIndex:end]

	// Determine next page token
	var nextPageToken *string
	if end < len(tasks) {
		token := fmt.Sprintf("task_%d", end)
		nextPageToken = &token
	}

	return c.JSON(api.TaskListResponse{
		TaskLogs:      &paginatedTasks,
		NextPageToken: nextPageToken,
	})
}

// GetTask gets a task from a workflow run.
func (m *Metis) GetTask(c *fiber.Ctx, runID string, taskID string) error {
	collection := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection)

	var workflow schema.WorkflowCollection
	err := collection.FindOne(context.Background(), bson.M{"run_id": runID}).Decode(&workflow)
	if err != nil {
		if errors.Is(err, mongo.ErrNoDocuments) {
			logger.L.Warn("workflow not found", "run_id", runID)
			statusCode := int32(fiber.StatusNotFound)
			errMsg := "Workflow not found"
			return c.Status(fiber.StatusNotFound).JSON(api.ErrorResponse{
				Msg:        &errMsg,
				StatusCode: &statusCode,
			})
		}
		logger.L.Error("failed to get workflow", "error", err, "run_id", runID)
		statusCode := int32(fiber.StatusInternalServerError)
		errMsg := "Failed to get workflow"
		return c.Status(fiber.StatusInternalServerError).JSON(api.ErrorResponse{
			Msg:        &errMsg,
			StatusCode: &statusCode,
		})
	}

	// Find the specific task by ID/name
	for _, task := range workflow.Workflow.Tasks {
		if task.Name != nil && *task.Name == taskID {
			return c.JSON(task)
		}
	}

	// Task not found
	logger.L.Warn("task not found", "run_id", runID, "task_id", taskID)
	statusCode := int32(fiber.StatusNotFound)
	errMsg := "Task not found"
	return c.Status(fiber.StatusNotFound).JSON(api.ErrorResponse{
		Msg:        &errMsg,
		StatusCode: &statusCode,
	})
}

// GetServiceInfo gets the service information.
func (m *Metis) GetServiceInfo(c *fiber.Ctx) error {
	createdAt, parseErr := time.Parse(time.RFC3339, "2025-01-01T00:00:00Z")
	if parseErr != nil {
		logger.L.Error("failed to parse created at", "error", parseErr)
		createdAt = time.Now()
	}
	updatedAt, parseErr := time.Parse(time.RFC3339, "2025-01-01T00:00:00Z")
	if parseErr != nil {
		logger.L.Error("failed to parse updated at", "error", parseErr)
		updatedAt = time.Now()
	}
	serviceInfo := api.ServiceInfo{
		Id:   "metis",
		Name: "Metis Workflow Execution Service",
		Type: api.ServiceType{
			Group:    "org.ga4gh",
			Artifact: "wes",
			Version:  "1.1.0",
		},
		Description: stringPtr("Workflow Execution Service for running computational workflows"),
		Organization: struct {
			Name string `json:"name"`
			//nolint:revive,staticcheck // ignore var-naming: struct field Url should be URL (revive, staticcheck)
			Url string `json:"url"`
		}{
			Name: "Metis",
			Url:  "https://github.com/jaeaeich/metis",
		},
		ContactUrl:                      stringPtr("https://github.com/jaeaeich/metis"),
		DocumentationUrl:                stringPtr("https://github.com/jaeaeich/metis/blob/main/README.md"),
		CreatedAt:                       &createdAt,
		UpdatedAt:                       &updatedAt,
		Environment:                     stringPtr("production"),
		Version:                         "1.0.0",
		AuthInstructionsUrl:             "",
		SupportedWesVersions:            []string{"1.0.0"},
		SupportedFilesystemProtocols:    []string{"http", "https", "file", "s3"},
		WorkflowTypeVersions:            map[string]api.WorkflowTypeVersion{},
		WorkflowEngineVersions:          map[string]api.WorkflowEngineVersion{},
		DefaultWorkflowEngineParameters: []api.DefaultWorkflowEngineParameter{},
		SystemStateCounts: map[string]int64{
			"QUEUED":   0,
			"RUNNING":  0,
			"COMPLETE": 0,
			"ERROR":    0,
		},
		Tags: map[string]string{
			"environment": "production",
			"version":     "1.0.0",
		},
	}

	return c.JSON(serviceInfo)
}

// Helper function to create string pointers.
func stringPtr(s string) *string {
	return &s
}
