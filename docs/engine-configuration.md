# Engine Configuration

Engine behaviour in Metis is entirely defined by `engine.yaml`. No code changes are required to add a new engine variant or adjust how parameters are handled.

## What is an Engine?

An engine is a combination of:

1. A **binary** (`metis-engine-generic`) that handles NATS subscription, process execution, log capture, and state transitions
2. A **config file** (`engine.yaml`) that defines how to build CLI commands, validate parameters, and lay out the working directory
3. An optional **trait implementation** for engine-specific result parsing and task log extraction

The generic binary reads `engine.yaml` at startup and handles the rest automatically.

## Starting an Engine

```bash
metis-engine-generic server --engine-config /etc/metis/engine-config.yaml
```

Set the three required environment variables:

```bash
DATABASE_URL=postgres://user:pass@host:5432/metis
REDIS_URL=redis://:pass@host:6379
NATS_URL=nats://user:pass@host:4222
```

## Full engine.yaml Reference

```yaml
version: "1.0.0"

engine:
  name: Nextflow               # Engine name — must match workflow_engine in RunRequest
  version: "25.10.4"           # Engine binary version
  backend: local               # Execution backend (local | k8s)

  workflowTypes:               # Accepted workflow_type values
    - NFL

  workflowTypeVersions:        # Accepted workflow_type_version values
    - DSL2

  # -------------------------------------------------------------------
  # Command Template
  # -------------------------------------------------------------------
  # Variables substituted at runtime:
  #   {workflow_path}   — absolute path to the workflow file/URL
  #   {engine_params}   — all validated engine parameters as CLI flags
  #   {workflow_params} — workflow parameters (inline or via params file)
  #   {workdir}         — the run's working directory
  #   {run_id}          — the run UUID
  #   {user_id}         — the submitting user identifier
  commandTemplate: "nextflow run {workflow_path} {engine_params} {workflow_params}"

  # -------------------------------------------------------------------
  # Workflow Parameters Style
  # -------------------------------------------------------------------
  workflowParams:
    style:
      method: file            # inline | file
      # inline: params passed as --key value on the CLI
      # file:   params written to a params file, path substituted into command
      prefix: "--"
      separator: " "
      keyValueFormat: "{key}={value}"

  # -------------------------------------------------------------------
  # Engine Parameters
  # -------------------------------------------------------------------
  engineParams:
    unknownParamsBehavior: reject   # reject | ignore | passthrough

    validatedParams:
      - names: ["profile", "-profile", "profiles"]   # All accepted aliases
        cliFlag: "-profile"                           # Flag written to CLI
        type: list                                    # string | int | bool | list
        default: ["docker"]
        strictDefault: false      # true = always use default even if client sends value
        required: false
        listSeparator: ","
        validate:
          - type: enum
            allowed: ["docker", "local", "k8s", "aws", "gcp", "slurm"]
            message: "Invalid profile"

      - names: ["resume", "-resume"]
        cliFlag: "-resume"
        type: bool
        default: false
        booleanStyle: flag        # flag = presence-only (-resume), value = -resume true

      - names: ["work-dir", "-work-dir", "-w"]
        cliFlag: "-work-dir"
        type: string
        required: false
        validate:
          - type: regex
            pattern: "^/[a-zA-Z0-9/_.-]+$"
            message: "Invalid work directory path"

      - names: ["max-cpus", "maxCpus"]
        cliFlag: "--max-cpus"
        type: int
        validate:
          - type: range
            min: 1
            max: 256

      - names: ["max-memory", "maxMemory"]
        cliFlag: "--max-memory"
        type: string
        default: "32GB"
        validate:
          - type: regex
            pattern: "^[0-9]+(\\.[0-9]+)?(KB|MB|GB|TB)$"
            message: "Memory must be like 16GB, 1.5TB"

      - names: ["config", "-c"]
        cliFlag: "-c"
        type: string
        validate:
          - type: file_exists          # File must exist on the engine host
          - type: file_extension
            allowed: [".config", ".cfg", ".nf"]

      # Sensitive params — value redacted from logs, passed via env var
      - names: ["tower-token", "access-token"]
        cliFlag: "-with-tower"
        type: string
        sensitive: true              # Value never written to log_lines
        envVar: "TOWER_ACCESS_TOKEN" # Set as environment variable instead of CLI flag
        validate:
          - type: regex
            pattern: "^[a-zA-Z0-9_.-]{10,}$"

  # -------------------------------------------------------------------
  # Denied Parameters
  # -------------------------------------------------------------------
  deniedParams:
    - pattern: "-with-docker"
      type: exact           # exact | regex
      reason: "Use profile parameter instead"

    - pattern: ".*[;&|`$()].*"
      type: regex
      reason: "Shell metacharacters not allowed"

  # -------------------------------------------------------------------
  # Ignored Parameters
  # -------------------------------------------------------------------
  # Parameters that are silently dropped (not validated, not passed to CLI)
  ignoredParams:
    - "debug"
    - "dry-run"

# -------------------------------------------------------------------
# Run Working Directory
# -------------------------------------------------------------------
runs:
  workdir:
    base: "/var/wes/runs"
    pattern: "{user_id}/{run_id}"    # Resolved path: /var/wes/runs/{user_id}/{run_id}
    subdirs:
      logs: "logs"
      outputs: "outputs"
      work: "work"
      tmp: "tmp"
```

## Parameter Fields Reference

| Field | Type | Required | Description |
| ----- | ---- | -------- | ----------- |
| `names` | `[string]` | yes | All accepted aliases. First is canonical. |
| `cliFlag` | `string` | yes | Flag written to the CLI command |
| `type` | `string` | yes | `string`, `int`, `bool`, `list` |
| `default` | any | no | Value used when client omits the parameter |
| `strictDefault` | `bool` | no | If true, always use default, ignore client value |
| `required` | `bool` | no | Reject run if missing and no default |
| `listSeparator` | `string` | no | Separator for `list` type (default `,`) |
| `booleanStyle` | `string` | no | `flag` (presence) or `value` (`-flag true`) |
| `sensitive` | `bool` | no | Redact value from logs; pass via `envVar` |
| `envVar` | `string` | no | Environment variable to set (requires `sensitive: true`) |
| `validate` | `[ValidationSpec]` | no | List of validation rules |

## Validation Types

| Type | Fields | Description |
| ---- | ------ | ----------- |
| `enum` | `allowed: [string]`, `message` | Value must be in allowed list |
| `regex` | `pattern: string`, `message` | Value must match regex |
| `range` | `min: int`, `max: int` | Numeric value must be within range |
| `file_exists` | — | File must exist on the engine host filesystem |
| `file_extension` | `allowed: [string]` | File path must end with one of the allowed extensions |

## Denied Parameters

Denied params are evaluated against the raw keys sent by the client before alias resolution.

| Field | Values | Description |
| ----- | ------ | ----------- |
| `pattern` | `string` | The pattern to match against |
| `type` | `exact`, `regex` | Match mode |
| `reason` | `string` | Error message returned to client |

## workflowParams Methods

| Method | Behaviour |
| ------ | --------- |
| `inline` | Parameters appended directly to the CLI command as flags |
| `file` | Parameters written to a temporary params file; path substituted into `{workflow_params}` in the command template |

## Registering a New Engine

1. Write an `engine.yaml` for the new engine.
2. Start `metis-engine-generic`:

  ```bash
  metis-engine-generic server --engine-config /path/to/engine.yaml
  ```

3. The engine registers its heartbeat in Valkey — it will appear in `GET /service-info`.
4. Submit runs with `workflow_engine` matching `engine.name` in the config.

For engines requiring custom result parsing or task log extraction, implement the `Engine` trait in a new crate and compile a custom binary.
