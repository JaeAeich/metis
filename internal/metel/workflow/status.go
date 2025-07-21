// Package workflow provides the workflow execution logic for metel.
package workflow

// JobStatus represents the final state of a Kubernetes job.
type JobStatus int

const (
	// JobSucceeded indicates that the job completed successfully.
	JobSucceeded JobStatus = iota
	// JobFailedCommand indicates that the job failed due to the command inside the container exiting with a non-zero status.
	JobFailedCommand
	// JobFailedSystem indicates that the job failed due to a Kubernetes system error (e.g., scheduling, image pull).
	JobFailedSystem
)

// JobResult holds the outcome of a workflow job execution.
type JobResult struct {
	Logs    string
	Message string
	Status  JobStatus
}
