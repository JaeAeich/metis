// Package handlers provides the handlers for the Metis API.
package handlers

import (
	"fmt"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"

	api "github.com/jaeaeich/metis/internal/api/generated"
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
	return c.Status(fiber.StatusOK).JSON(api.RunId{RunId: &runID})
}

// GetRunLog gets the log for a workflow run.
func (m *Metis) GetRunLog(c *fiber.Ctx, runID string) error {
	state := api.RUNNING
	return c.JSON(api.RunLog{
		RunId: &runID,
		State: &state,
	})
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
