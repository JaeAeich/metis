package staging

import (
	"context"
	"fmt"
	"os"
	"path"
	"path/filepath"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"

	root "github.com/jaeaeich/metis/internal/config"
	"github.com/jaeaeich/metis/internal/logger"
	"github.com/jaeaeich/metis/internal/metel/proto"
)

// S3Provider is a staging provider for AWS S3.
type S3Provider struct{}

// GetURI returns the S3 URI for a given run ID.
func (p *S3Provider) GetURI(runID string) (string, error) {
	stagingPath := path.Join(root.Cfg.Metel.Staging.Prefix, runID)
	return fmt.Sprintf("s3://%s/%s", root.Cfg.Metel.Staging.Bucket, stagingPath), nil
}

// UploadFile uploads a file to S3.
func (p *S3Provider) UploadFile(localPath, remotePath string, stagingInfo *proto.StagingInfo) error {
	client, err := newS3Client(stagingInfo)
	if err != nil {
		return err
	}

	//nolint:gosec // The file path is controlled by the system and not user input.
	file, err := os.Open(localPath)
	if err != nil {
		return fmt.Errorf("failed to open file %s: %w", localPath, err)
	}
	defer func() {
		if closeErr := file.Close(); closeErr != nil {
			logger.L.Error("failed to close file", "path", localPath, "error", closeErr)
		}
	}()

	_, err = client.PutObject(context.TODO(), &s3.PutObjectInput{
		Bucket: aws.String(root.Cfg.Metel.Staging.Bucket),
		Key:    aws.String(remotePath),
		Body:   file,
	})
	if err != nil {
		return fmt.Errorf("failed to upload file %s to S3: %w", localPath, err)
	}
	return nil
}

// UploadDir uploads a directory to S3.
func (p *S3Provider) UploadDir(localPath, remotePath string, stagingInfo *proto.StagingInfo) error {
	client, err := newS3Client(stagingInfo)
	if err != nil {
		return err
	}

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
				Bucket: aws.String(root.Cfg.Metel.Staging.Bucket),
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

func newS3Client(stagingInfo *proto.StagingInfo) (*s3.Client, error) {
	awsRegion, ok := stagingInfo.Parameters["AWS_REGION"]
	if !ok {
		awsRegion = "us-east-1"
	}
	cfg, err := config.LoadDefaultConfig(
		context.TODO(),
		config.WithRegion(awsRegion),
		config.WithCredentialsProvider(
			credentials.NewStaticCredentialsProvider(
				stagingInfo.Parameters["AWS_ACCESS_KEY_ID"],
				stagingInfo.Parameters["AWS_SECRET_ACCESS_KEY"],
				"",
			),
		),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to load AWS config: %w", err)
	}

	client := s3.NewFromConfig(cfg, func(o *s3.Options) {
		if endpoint, ok := stagingInfo.Parameters["AWS_ENDPOINT_URL"]; ok {
			o.BaseEndpoint = aws.String(endpoint)
			o.UsePathStyle = true
		}
	})
	return client, nil
}
