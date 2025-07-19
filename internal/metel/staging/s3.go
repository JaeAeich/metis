package staging

import (
	"fmt"
	"path"

	"github.com/jaeaeich/metis/internal/config"
)

// S3Provider is a staging provider for AWS S3.
type S3Provider struct{}

// GetURL returns the S3 URL for a given run ID.
func (p *S3Provider) GetURL(runID string) (string, error) {
	stagingPath := path.Join(config.Cfg.Metel.Staging.Prefix, runID)
	return fmt.Sprintf("s3://%s/%s", config.Cfg.Metel.Staging.Bucket, stagingPath), nil
}
