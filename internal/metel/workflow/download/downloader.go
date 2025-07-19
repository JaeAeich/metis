// Package download provides the downloader interface and implementations for
// downloading workflows from various sources.
package download

// Downloader is an interface for downloading workflows from various sources.
type Downloader interface {
	// Download downloads a workflow from a given URL to a destination directory
	// and returns the path to the primary workflow descriptor. If the primary
	// descriptor is not changed, it should return an empty string.
	Download(url string, destination string) (string, error)
}
