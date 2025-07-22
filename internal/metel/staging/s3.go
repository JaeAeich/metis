package staging

import (
	"context"
	"fmt"
	"os"
	"path"
	"path/filepath"

	"github.com/aws/aws-sdk-go-v2/aws"
	awsCfg "github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/s3"

	"github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
)

// S3Provider is a staging provider for AWS S3.
type S3Provider struct{}

// GetURL returns the S3 URL for a given run ID.
func (p *S3Provider) GetURL(runID string) (string, error) {
	stagingPath := path.Join(config.Cfg.Metel.Staging.Prefix, runID)
	return fmt.Sprintf("s3://%s/%s", config.Cfg.Metel.Staging.Bucket, stagingPath), nil
}

// UploadDir uploads a directory to S3.
func (p *S3Provider) UploadDir(localPath, remotePath string) error {
	cfg, err := awsCfg.LoadDefaultConfig(context.TODO())
	if err != nil {
		return fmt.Errorf("failed to load AWS config: %w", err)
	}

	client := s3.NewFromConfig(cfg, func(o *s3.Options) {
		if os.Getenv("AWS_ENDPOINT_URL") != "" {
			o.UsePathStyle = true
		}
	})

	return filepath.Walk(localPath, func(filePath string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() {
			relPath, err := filepath.Rel(localPath, filePath)
			if err != nil {
				return fmt.Errorf("failed to get relative path: %w", err)
			}

			//nolint:gosec //The file path is controlled by the system and not user input.
			file, err := os.Open(filePath)
			if err != nil {
				return fmt.Errorf("failed to open file %s: %w", filePath, err)
			}
			defer func() {
				if closeErr := file.Close(); closeErr != nil {
					logger.L.Error("failed to close file", "path", filePath, "error", closeErr)
				}
			}()

			_, err = client.PutObject(context.TODO(), &s3.PutObjectInput{
				Bucket: aws.String(config.Cfg.Metel.Staging.Bucket),
				Key:    aws.String(path.Join(remotePath, relPath)),
				Body:   file,
			})
			if err != nil {
				return fmt.Errorf("failed to upload file %s to S3: %w", filePath, err)
			}
		}
		return nil
	})
}
