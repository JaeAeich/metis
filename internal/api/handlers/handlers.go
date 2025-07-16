// Package handlers provides the handlers for the Metis API.
package handlers

import (
	"context"
	"fmt"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	api "github.com/jaeaeich/metis/internal/api/generated"
	run "github.com/jaeaeich/metis/internal/api/handlers/workflow"
	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/logger"
)

// Metis is our handler struct.
type Metis struct{}

// Make sure we conform to the generated server interface.
var _ api.ServerInterface = (*Metis)(nil)

// ListRuns lists all the workflow runs.
func (m *Metis) ListRuns(c *fiber.Ctx, params api.ListRunsParams) error {
	runs := make([]api.RunListResponse_Runs_Item, 0)
	runID := uuid.New().String()
	state := api.UNKNOWN
	runStatus := api.RunStatus{
		RunId: runID,
		State: &state,
	}
	var runItem api.RunListResponse_Runs_Item
	err := runItem.FromRunStatus(runStatus)
	if err != nil {
		logger.L.Error("failed to convert run status to run item", "error", err)
		return err
	}

	runs = append(runs, runItem)

	nextPageToken := ""
	if params.PageSize != nil && *params.PageSize > 0 {
		nextPageToken = "some_next_page_token"
	}

	return c.JSON(api.RunListResponse{
		Runs:          &runs,
		NextPageToken: &nextPageToken,
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
	var runLog api.RunLog
	err := clients.DB.Database("metis").Collection("workflows").FindOne(context.Background(), map[string]string{"run_id": runID}).Decode(&runLog)
	if err != nil {
		logger.L.Error("failed to get run log", "error", err, "run_id", runID)
		return err
	}
	return c.JSON(runLog)
}

// CancelRun cancels a workflow run.
func (m *Metis) CancelRun(c *fiber.Ctx, runID string) error {
	return c.JSON(api.RunId{RunId: &runID})
}

// GetRunStatus gets the status of a workflow run.
func (m *Metis) GetRunStatus(c *fiber.Ctx, runID string) error {
	state := api.RUNNING
	return c.JSON(api.RunStatus{
		RunId: runID,
		State: &state,
	})
}

// ListTasks lists the tasks for a workflow run.
func (m *Metis) ListTasks(c *fiber.Ctx, runID string, params api.ListTasksParams) error {
	tasks := make([]api.TaskLog, 0)
	tasks = append(tasks, api.TaskLog{
		Name: &runID,
	})
	return c.JSON(api.TaskListResponse{
		TaskLogs: &tasks,
	})
}

// GetTask gets a task from a workflow run.
func (m *Metis) GetTask(c *fiber.Ctx, runID string, taskID string) error {
	taskName := fmt.Sprintf("%s-%s", runID, taskID)
	return c.JSON(api.TaskLog{
		Name: &taskName,
	})
}

// GetServiceInfo gets the service information.
func (m *Metis) GetServiceInfo(c *fiber.Ctx) error {
	return c.JSON(api.ServiceInfo{
		Id: "metis",
	})
}
