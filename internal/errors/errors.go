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

// ErrTRSMetaData is returned when getting files metadata from TRS fails.
var ErrTRSMetaData = errors.New("failed to get files metadata from TRS")

// ErrTRSReadBody is returned when reading response body from TRS fails.
var ErrTRSReadBody = errors.New("failed to read response body from TRS")

// ErrTRSUnmarshal is returned when unmarshalling response body from TRS fails.
var ErrTRSUnmarshal = errors.New("failed to unmarshal response body from TRS")

// ErrTRSURL is returned when the TRS URL is invalid.
var ErrTRSURL = errors.New("invalid TRS URL")

// ErrTRSURLFormat is returned when the TRS URL format is invalid.
var ErrTRSURLFormat = errors.New("invalid TRS URL format")

// ErrNoFilesFound is returned when no files are found for a given tool.
var ErrNoFilesFound = errors.New("no files found for the given tool, version and descriptor type")

// ErrDirCreation is returned when creating a directory fails.
var ErrDirCreation = errors.New("failed to create directory")

// ErrFileCreation is returned when creating a file fails.
var ErrFileCreation = errors.New("failed to create file")

// ErrFileDownload is returned when downloading a file fails.
var ErrFileDownload = errors.New("failed to download file")

// ErrFileWrite is returned when writing to a file fails.
var ErrFileWrite = errors.New("failed to write to file")

// ErrNoFileInResponse is returned when no file is found in TRS response.
var ErrNoFileInResponse = errors.New("no file found in TRS response")
