package download

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/jaeaeich/metis/internal/errors"
)

// FileDownloader is a downloader for file URLs.
type FileDownloader struct{}

// Download checks for the existence of a local file specified by a file:// URL.
// This protocol expects that the user has already put the file as
// workflow_attachment in the WES request, which means it should already
// be present at the mount, ie destination.
// Example: url: file://my-file
func (d *FileDownloader) Download(url string, destination string, descriptorType string) (string, error) {
	fileName := strings.TrimPrefix(url, "file://")

	filePath := filepath.Join(destination, fileName)

	// Security check to prevent path traversal.
	cleanFilePath := filepath.Clean(filePath)
	if !strings.HasPrefix(cleanFilePath, filepath.Clean(destination)) {
		return "", fmt.Errorf("%w: access to %s is not allowed", errors.ErrInvalidFilePath, cleanFilePath)
	}

	if _, err := os.Stat(cleanFilePath); os.IsNotExist(err) {
		return "", fmt.Errorf("%w: %s", errors.ErrFileNotFound, cleanFilePath)
	} else if err != nil {
		return "", fmt.Errorf("error checking file %s: %w", cleanFilePath, err)
	}

	return cleanFilePath, nil
}
