// Package staging provides an interface for creating remote staging areas.
package staging

import (
	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/errors"
)

// Provider is an interface for creating remote staging area URLs.
type Provider interface {
	// GetURL returns the remote staging area URL for a given run ID.
	GetURL(runID string) (string, error)
	// UploadDir uploads a directory to the remote staging area.
	UploadDir(localPath, remotePath string) error
}

// GetProvider returns a staging provider based on the configuration.
//
//nolint:ireturn // Returning Downloader interface is intentional for factory pattern
func GetProvider() (Provider, error) {
	switch config.Cfg.Metel.Staging.Type {
	case "s3":
		return &S3Provider{}, nil
	default:
		return nil, errors.ErrUnsupportedStagingProviderType
	}
}
