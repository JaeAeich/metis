// Package schema provides the database schema for the Metis application.
package schema

import (
	"time"

	"go.mongodb.org/mongo-driver/bson/primitive"

	api "github.com/jaeaeich/metis/internal/api/generated"
)

// WorkflowCollection represents the workflow collection structure in MongoDB.
type WorkflowCollection struct {
	CreatedAt time.Time          `bson:"created_at" json:"created_at"`
	UpdatedAt time.Time          `bson:"updated_at" json:"updated_at"`
	RunID     string             `bson:"run_id" json:"run_id"`
	Workflow  WorkflowData       `bson:"workflow" json:"workflow"`
	UserID    int                `bson:"user_id" json:"user_id"`
	ID        primitive.ObjectID `bson:"_id,omitempty" json:"id,omitempty"`
}

// WorkflowData contains the workflow execution data.
type WorkflowData struct {
	RunLog *api.RunLog   `bson:"run_log,omitempty" json:"run_log,omitempty"`
	Tasks  []api.TaskLog `bson:"tasks,omitempty" json:"tasks,omitempty"`
}

// ServiceCollection represents the service collection structure in MongoDB.
type ServiceCollection struct {
	CreatedAt   time.Time          `bson:"created_at" json:"created_at"`
	UpdatedAt   time.Time          `bson:"updated_at" json:"updated_at"`
	ServiceInfo *api.ServiceInfo   `bson:"service_info,omitempty" json:"service_info,omitempty"`
	UserID      int                `bson:"user_id" json:"user_id"`
	ID          primitive.ObjectID `bson:"_id,omitempty" json:"id,omitempty"`
}

// NewWorkflowCollection creates a new workflow collection document with default values.
func NewWorkflowCollection(runID string) *WorkflowCollection {
	now := time.Now()
	return &WorkflowCollection{
		UserID:    -1, // Default value as requested
		RunID:     runID,
		Workflow:  WorkflowData{},
		CreatedAt: now,
		UpdatedAt: now,
	}
}

// NewServiceCollection creates a new service collection document with default values.
func NewServiceCollection() *ServiceCollection {
	now := time.Now()
	return &ServiceCollection{
		UserID:    -1, // Default value as requested
		CreatedAt: now,
		UpdatedAt: now,
	}
}
