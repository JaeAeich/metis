// Package errors provides custom errors for the application.
package errors

import "errors"

// ErrNoSuitablePlugin is returned when no suitable plugin is found.
var ErrNoSuitablePlugin = errors.New("no suitable plugin found")

// ErrUnsupportedProtocol is returned when the protocol is unsupported.
var ErrUnsupportedProtocol = errors.New("unsupported protocol")

// ErrInvalidFilePath is returned when the file path is invalid.
var ErrInvalidFilePath = errors.New("invalid file path")

// ErrFileNotFound is returned when a file is not found.
var ErrFileNotFound = errors.New("file not found")

// ErrUnsupportedStagingProviderType is returned when the staging provider type is unsupported.
var ErrUnsupportedStagingProviderType = errors.New("unsupported staging provider type")

// ErrJobFailed is returned when a Kubernetes job fails.
var ErrJobFailed = errors.New("job failed")

// ErrJobNotFinished is returned when a job is still running or in an unknown state.
var ErrJobNotFinished = errors.New("job is still running or in an unknown state")
