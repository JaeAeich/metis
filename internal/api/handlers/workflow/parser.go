package run

import (
	"encoding/json"

	"github.com/gofiber/fiber/v2"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/logger"
)

// ParseRunRequest parses the multipart form and returns a RunRequest struct.
func ParseRunRequest(c *fiber.Ctx) (*api.RunRequest, error) {
	runRequest := &api.RunRequest{}

	form, err := c.MultipartForm()
	if err != nil {
		return nil, err
	}

	// Helper to get form value safely
	getFormValue := func(key string) string {
		if values, exists := form.Value[key]; exists && len(values) > 0 {
			return values[0]
		}
		return ""
	}

	// Helper for setting string pointers
	setStrPtr := func(s string) *string {
		if s == "" {
			return nil
		}
		return &s
	}

	// Map form values to struct
	runRequest.WorkflowEngine = setStrPtr(getFormValue("workflow_engine"))
	runRequest.WorkflowUrl = getFormValue("workflow_url")
	runRequest.WorkflowType = getFormValue("workflow_type")
	runRequest.WorkflowTypeVersion = getFormValue("workflow_type_version")
	runRequest.WorkflowEngineVersion = setStrPtr(getFormValue("workflow_engine_version"))

	// The WES spec expects these fields to be JSON strings, so we unmarshal them.
	if paramsStr := getFormValue("workflow_params"); paramsStr != "" {
		var params map[string]interface{}
		if unmarshalErr := json.Unmarshal([]byte(paramsStr), &params); unmarshalErr == nil {
			runRequest.WorkflowParams = &params
		} else {
			logger.L.Warn("failed to unmarshal workflow_params", "error", unmarshalErr)
		}
	}
	if engineParamsStr := getFormValue("workflow_engine_parameters"); engineParamsStr != "" {
		var params map[string]string
		if unmarshalErr := json.Unmarshal([]byte(engineParamsStr), &params); unmarshalErr == nil {
			runRequest.WorkflowEngineParameters = &params
		} else {
			logger.L.Warn("failed to unmarshal workflow_engine_parameters", "error", unmarshalErr)
		}
	}
	if tagsStr := getFormValue("tags"); tagsStr != "" {
		var tags map[string]string
		if unmarshalErr := json.Unmarshal([]byte(tagsStr), &tags); unmarshalErr == nil {
			runRequest.Tags = &tags
		} else {
			logger.L.Warn("failed to unmarshal tags", "error", unmarshalErr)
		}
	}

	return runRequest, nil
}
