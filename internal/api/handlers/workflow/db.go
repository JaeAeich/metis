// Package run has business logic for creating and viewing runs
package run

import (
	"context"
	"time"

	api "github.com/jaeaeich/metis/internal/api/generated"
	"github.com/jaeaeich/metis/internal/clients"
	"github.com/jaeaeich/metis/internal/config"
)

// InsertRunLog inserts a new run log into the database.
func InsertRunLog(runID string, runRequest *api.RunRequest) error {
	runLog := api.RunLog{
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
	_, err := clients.DB.Database(config.Cfg.Mongo.Database).Collection(config.Cfg.Mongo.WorkflowCollection).InsertOne(context.Background(), &runLog)
	return err
}
