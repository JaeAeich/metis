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

1. The engine registers its heartbeat in Valkey — it will appear in `GET /service-info`.
2. Submit runs with `workflow_engine` matching `engine.name` in the config.

For engines requiring custom result parsing or task log extraction, implement the `Engine` trait in a new crate and compile a custom binary.
