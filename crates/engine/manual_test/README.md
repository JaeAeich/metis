# Manual Test - Metis Engine

This directory contains test files for manually testing the Metis Engine command building pipeline.

## Files

- `engine.yaml` - Engine configuration for Nextflow-style workflows
- `request.json` - Sample WES request for testing

## Installation

From the repository root:

```bash
# Build the engine binary
cargo build --release -p engine
```

## Running

### From workspace root (recommended)

```bash
# Run with test config and request
cargo run -p engine -- run --engine-config crates/engine/manual_test/engine.yaml -f crates/engine/manual_test/request.json
```

### Using the built binary

```bash
# From the build directory
./target/release/engine run --engine-config crates/engine/manual_test/engine.yaml -f crates/engine/manual_test/request.json
```

### Using JSON string instead of file

```bash
cargo run -- run --engine-config manual_test/engine.yaml '{"workflow_type":"NFL","workflow_type_version":"DSL2","workflow_url":"https://github.com/nf-core/rnaseq","workflow_params":{"input":"/data/samples.csv"},"workflow_engine_parameters":{"profile":"docker"}}'
```

## Expected Output

When running with the provided test files, you should see:

1. **Validation** - The WES request is validated against the engine config
2. **Staging Area Creation** - Working directories are created under `/tmp/metis_test/`
3. **Command Building** - Template variables are substituted
4. **Dry Run Output** - The complete command that would be executed

Example output:

```
════════════════════════════════════════════════════════════════════
DRY RUN - NoopEngine
════════════════════════════════════════════════════════════════════

Run ID:         019c962b-c2ed-70d2-83ce-76d581926b80
User ID:        cli
Workflow URL:   https://github.com/nf-core/rnaseq
Workflow Type:  NFL DSL2

Staging Area:
  workdir:   /tmp/metis_test/cli/019c962b-c2ed-70d2-83ce-76d581926b80
  logs:      /tmp/metis_test/cli/019c962b-c2ed-70d2-83ce-76d581926b80/logs
  outputs:   /tmp/metis_test/cli/019c962b-c2ed-70d2-83ce-76d581926b80/outputs
  work:      /tmp/metis_test/cli/019c962b-c2ed-70d2-83ce-76d581926b80/work

────────────────────────────────────────────────────────────────────
COMMAND (would execute):
────────────────────────────────────────────────────────────────────

cd /tmp/metis_test/cli/019c962b... && \
  TOWER_ACCESS_TOKEN=*** nextflow run https://github.com/nf-core/rnaseq \
  -profile docker,aws -resume --max-cpus 64 --max-memory 128GB ...

Environment Variables:
  TOWER_ACCESS_TOKEN=***REDACTED***

════════════════════════════════════════════════════════════════════
DRY RUN COMPLETE - No actual execution performed
════════════════════════════════════════════════════════════════════
```

## Testing Different Scenarios

### Test with different profiles

```bash
# Using multiple profiles (comma-separated)
cargo run -- run --engine-config manual_test/engine.yaml '{
  "workflow_type": "NFL",
  "workflow_type_version": "DSL2",
  "workflow_url": "https://github.com/nf-core/sarek",
  "workflow_params": {"input": "/data/samples.csv"},
  "workflow_engine_parameters": {"profile": "docker,singularity"}
}'
```

### Test validation errors

```bash
# Invalid profile (should fail validation)
cargo run -- run --engine-config manual_test/engine.yaml '{
  "workflow_type": "NFL",
  "workflow_type_version": "DSL2",
  "workflow_url": "https://github.com/nf-core/rnaseq",
  "workflow_engine_parameters": {"profile": "invalid_profile"}
}'

# Denied parameter (should fail security check)
cargo run -- run --engine-config manual_test/engine.yaml '{
  "workflow_type": "NFL",
  "workflow_type_version": "DSL2",
  "workflow_url": "https://github.com/nf-core/rnaseq",
  "workflow_engine_parameters": {"--privileged": "true"}
}'
```

### Test CPU range validation

```bash
# Valid CPU range (1-128)
cargo run -- run --engine-config manual_test/engine.yaml '{
  "workflow_type": "NFL",
  "workflow_type_version": "DSL2",
  "workflow_url": "https://github.com/nf-core/rnaseq",
  "workflow_engine_parameters": {"max-cpus": "64"}
}'

# Invalid CPU (exceeds max)
cargo run -- run --engine-config manual_test/engine.yaml '{
  "workflow_type": "NFL",
  "workflow_type_version": "DSL2",
  "workflow_url": "https://github.com/nf-core/rnaseq",
  "workflow_engine_parameters": {"max-cpus": "256"}
}'
```

## Cleanup

The test creates directories under `/tmp/metis_test/`. To clean up:

```bash
rm -rf /tmp/metis_test/
```

## CLI Options

```
engine run [OPTIONS] [WES_REQUEST]

Arguments:
  [WES_REQUEST]              WES request as JSON string

Options:
  -f, --file <FILE>           Path to WES request JSON file
      --engine-config <FILE>  Engine configuration file
  -h, --help                  Print help
  --log-level <LEVEL>         Log level: trace, debug, info, warn, error [default: info]
  --json-logging              Enable JSON formatted logs
```
