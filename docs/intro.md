# Introduction to Metis

<div align="center">
  <img src="/web-app-manifest-512x512.png" alt="Metis Logo" width="200"/>
</div>

## What is Metis?

Metis is a modern, flexible **Workflow Execution Service (WES)** designed to run
computational workflows on Kubernetes. It brings the power of cloud-native
computing to scientific and data-intensive workflows, acting as a smart manager
for your computational tasks.

## Plugin Architecture

The WES API provides a standardized way to execute workflows across different
engines. Metis takes a unique approach by embracing **modularity and separation
of concerns** through its plugin architecture.

Instead of implementing all workflow engine logic within a single service, Metis
leverages Kubernetes' natural isolation capabilities to create a distributed,
plugin-based system.

### How It Works

Each workflow engine is supported through a dedicated plugin that communicates
with Metis via a **gRPC contract**. This design provides several advantages:

- **Isolation**: Each plugin operates independently, ensuring stability and
  reliability
- **Modularity**: New workflow engines can be easily integrated through
  additional plugins
- **Maintainability**: Engine-specific functionality is organized into focused,
  purpose-built components

## Architecture Overview

### System Components

- **Metis**: The primary WES API server that handles client requests
- **METEL**: Metis Execution and Translation Enrichment Layer - orchestrates
  workflow execution
- **WE**: Workflow Executor - the actual execution environment for workflows
- **Plugin**: A gRPC server that provides engine-specific functionality

### Request Lifecycle

Metis uses a **multi-phase execution model** designed for reliability and clean
separation of concerns:

#### Phase 1: Workflow Preparation

When a workflow execution request is received, Metis creates a dedicated
Kubernetes job to:

- Download workflow descriptor files from the specified URL
- Set up a clean execution environment
- Prepare all necessary workflow dependencies

This ensures each workflow starts with a fresh, isolated environment.

#### Phase 2: Execution Planning

Once the workflow files are ready, Metel begins the **pre-launch lifecycle**:

1. **Plugin Consultation**: Metel queries the appropriate engine plugin for
   execution specifications
1. **Context Provision**: The plugin receives comprehensive context including:
   - Primary descriptor location (potentially from TRS API)
   - Workflow parameters and configuration
   - Execution environment details
1. **Command Generation**: The plugin returns engine-specific execution commands

#### Phase 3: Workflow Execution

Metel orchestrates the actual workflow execution by:

- Creating the workflow execution environment
- Monitoring job progress and status
- Capturing execution logs and outputs

#### Phase 4: Result Processing

After execution completion, Metel initiates the **post-launch lifecycle**:

1. **Log Aggregation**: All execution logs are uploaded to the remote staging
   area
1. **Response Mapping**: The plugin translates engine-specific outputs to
   WES-compliant responses
1. **Result Delivery**: Standardized results are returned to the client
