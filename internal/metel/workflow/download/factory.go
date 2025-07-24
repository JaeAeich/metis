// Package download provides a factory for creating downloaders based on the URL scheme.
package download

import (
	"fmt"
	"net/url"

	"github.com/jaeaeich/metis/internal/errors"
)

// GetDownloader returns a downloader based on the URL scheme.
//
//nolint:ireturn // Returning Downloader interface is intentional for factory pattern
func GetDownloader(rawURL string) (Downloader, error) {
	parsedURL, err := url.Parse(rawURL)
	if err != nil {
		return nil, fmt.Errorf("failed to parse URL: %w", err)
	}

	switch parsedURL.Scheme {
	case "http", "https":
		return &HTTPDownloader{}, nil
	case "file":
		return &FileDownloader{}, nil
	case "trs":
		return &TRSDownloader{}, nil
	default:
		return nil, errors.ErrUnsupportedProtocol
	}
}
