package download

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/jaeaeich/metis/internal/errors"
	"github.com/jaeaeich/metis/internal/logger"
)

// FileMetadata represents the structure of file metadata from TRS.
type FileMetadata struct {
	FileType string `json:"file_type"`
	Path     string `json:"path"`
	Checksum []struct {
		Checksum string `json:"checksum"`
		Type     string `json:"type"`
	} `json:"checksum"`
}

// FileDownloadMetadata represents the structure for file download metadata from TRS.
type FileDownloadMetadata struct {
	Content  string `json:"content"`
	URL      string `json:"url"`
	Checksum []struct {
		Checksum string `json:"checksum"`
		Type     string `json:"type"`
	} `json:"checksum"`
}

// TRSDownloader is the struct for the TRS downloader.
type TRSDownloader struct{}

// Download retrieves all workflow files from a TRS store to the destination directory.
// If the path in TRS is main.wdl or /main.wdl, it will be downloaded to the destination directory.
func (d *TRSDownloader) Download(url string, destination string, descriptorType string) (string, error) {
	// Parse TRS URL
	rest := strings.TrimPrefix(url, "trs://")
	parts := strings.Split(rest, "/")
	if len(parts) < 2 {
		logger.L.Error("Invalid TRS URL format", "url", url)
		return "", errors.ErrTRSURL
	}

	// Extract toolID and version - they are the last two parts
	version := parts[len(parts)-1]
	toolID := parts[len(parts)-2]
	trsServerURL := strings.Join(parts[:len(parts)-2], "/")
	logger.L.Debug("TRS server URL", "url", trsServerURL)
	logger.L.Debug("Tool ID", "toolID", toolID)
	logger.L.Debug("Version", "version", version)

	// Fetch files metadata using /tools/{id}/versions/{version_id}/{type}/files
	filesMetadataEndpoint := fmt.Sprintf("https://%s/tools/%s/versions/%s/%s/files", trsServerURL, toolID, version, descriptorType)
	logger.L.Debug("Files metadata endpoint", "url", filesMetadataEndpoint)

	req, err := http.NewRequestWithContext(context.Background(), http.MethodGet, filesMetadataEndpoint, nil)
	if err != nil {
		return "", fmt.Errorf("%w: %w", errors.ErrTRSMetaData, err)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("%w: %w", errors.ErrTRSMetaData, err)
	}
	defer func() {
		if closeErr := resp.Body.Close(); closeErr != nil {
			logger.L.Error("failed to close metadata response body", "error", closeErr)
		}
	}()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("%w: received status code %d", errors.ErrTRSMetaData, resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("%w: %w", errors.ErrTRSReadBody, err)
	}

	var files []FileMetadata
	if err := json.Unmarshal(body, &files); err != nil {
		return "", fmt.Errorf("%w: %w", errors.ErrTRSUnmarshal, err)
	}

	if len(files) == 0 {
		return "", errors.ErrNoFilesFound
	}

	primaryDescriptorPath := ""

	// For each file, get the descriptor content using /tools/{id}/versions/{version_id}/{type}/descriptor/{relative_path}
	for _, file := range files {
		if file.Path == "" {
			logger.L.Warn("Skipping file with empty path", "file_type", file.FileType)
			continue
		}

		if err := downloadFileDescriptor(trsServerURL, toolID, version, descriptorType, file, destination); err != nil {
			return "", err
		}

		if file.FileType == "PRIMARY_DESCRIPTOR" {
			primaryDescriptorPath = file.Path
		}
	}

	if primaryDescriptorPath == "" {
		return "", errors.ErrNoFileInResponse
	}

	return filepath.Join(destination, primaryDescriptorPath), nil
}

// downloadFileDescriptor downloads a file by calling the TRS descriptor endpoint.
// It handles both direct content and URL-based downloads.
func downloadFileDescriptor(trsServerURL, toolID, version, descriptorType string, file FileMetadata, destination string) error {
	// Call the descriptor endpoint: /tools/{id}/versions/{version_id}/{type}/descriptor/{relative_path}
	descriptorEndpoint := fmt.Sprintf("https://%s/tools/%s/versions/%s/%s/descriptor/%s", trsServerURL, toolID, version, descriptorType, file.Path)
	logger.L.Debug("Descriptor endpoint", "url", descriptorEndpoint)

	req, err := http.NewRequestWithContext(context.Background(), http.MethodGet, descriptorEndpoint, nil)
	if err != nil {
		return fmt.Errorf("%w: %w", errors.ErrFileDownload, err)
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("%w: %w", errors.ErrFileDownload, err)
	}
	defer func() {
		if closeErr := resp.Body.Close(); closeErr != nil {
			logger.L.Error("failed to close descriptor response body", "error", closeErr)
		}
	}()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("%w: received status code %d", errors.ErrFileDownload, resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("%w: %w", errors.ErrTRSReadBody, err)
	}

	var descriptor FileDownloadMetadata
	if unmarshErr := json.Unmarshal(body, &descriptor); unmarshErr != nil {
		return fmt.Errorf("%w: %w", errors.ErrTRSUnmarshal, unmarshErr)
	}

	// Create the destination path
	destPath := filepath.Join(destination, file.Path)

	// Ensure directory exists for the file
	if makeDirErr := os.MkdirAll(filepath.Dir(destPath), 0o755); makeDirErr != nil { //nolint:gosec // File path is constructed from TRS metadata
		return fmt.Errorf("%w: %w", errors.ErrDirCreation, makeDirErr)
	}

	// Create output file
	outFile, err := os.Create(destPath) //nolint:gosec // File path is constructed from TRS metadata
	if err != nil {
		return fmt.Errorf("%w: %w", errors.ErrFileCreation, err)
	}
	defer func() {
		if closeErr := outFile.Close(); closeErr != nil {
			logger.L.Error("failed to close output file", "error", closeErr)
		}
	}()

	// If content is present, write it directly
	if descriptor.Content != "" {
		logger.L.Debug("Writing content directly to file", "path", destPath)
		if _, writeErr := outFile.WriteString(descriptor.Content); writeErr != nil {
			return fmt.Errorf("%w: %w", errors.ErrFileWrite, writeErr)
		}
		return nil
	}

	// Otherwise, download from URL
	if descriptor.URL == "" {
		return fmt.Errorf("%w: %w", errors.ErrFileDownload, err)
	}

	logger.L.Debug("Downloading file from URL", "url", descriptor.URL, "to", destPath)

	urlReq, err := http.NewRequestWithContext(context.Background(), http.MethodGet, descriptor.URL, nil)
	if err != nil {
		return fmt.Errorf("%w: %w", errors.ErrFileDownload, err)
	}

	urlResp, err := http.DefaultClient.Do(urlReq)
	if err != nil {
		return fmt.Errorf("%w: %w", errors.ErrFileDownload, err)
	}
	defer func() {
		if closeErr := urlResp.Body.Close(); closeErr != nil {
			logger.L.Error("failed to close URL response body", "error", closeErr)
		}
	}()

	if urlResp.StatusCode != http.StatusOK {
		return fmt.Errorf("%w: received status code %d", errors.ErrFileDownload, urlResp.StatusCode)
	}

	// Write file content from URL
	if _, err := io.Copy(outFile, urlResp.Body); err != nil {
		return fmt.Errorf("%w: %w", errors.ErrFileWrite, err)
	}

	return nil
}
