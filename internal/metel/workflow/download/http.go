package download

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
)

// HTTPDownloader is a downloader for HTTP URLs.
type HTTPDownloader struct{}

// Download downloads a file from an HTTP URL.
// Example: url: https://example.com/my-file
func (d *HTTPDownloader) Download(url string, destination string, descriptorType string) (string, error) {
	req, err := http.NewRequestWithContext(context.Background(), http.MethodGet, url, nil)
	if err != nil {
		return "", err
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", err
	}
	defer func() {
		if closeErr := resp.Body.Close(); closeErr != nil {
			fmt.Printf("failed to close response body: %v\n", closeErr)
		}
	}()

	fileName := filepath.Base(req.URL.Path)
	filePath := filepath.Join(destination, fileName)

	//nolint:gosec // We are not using this file for anything other than the workflow.
	out, err := os.Create(filePath)
	if err != nil {
		return "", err
	}
	defer func() {
		if closeErr := out.Close(); closeErr != nil {
			fmt.Printf("failed to close file: %v\n", closeErr)
		}
	}()

	_, err = io.Copy(out, resp.Body)
	if err != nil {
		return "", err
	}

	return filePath, nil
}
