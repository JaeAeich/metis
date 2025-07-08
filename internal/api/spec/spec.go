// Package spec provides the OpenAPI specification for the Metis API.
package spec

import _ "embed"

// Spec is the OpenAPI specification for the Metis API.
//
//go:embed 3a832ab.wes.yaml
var Spec []byte
