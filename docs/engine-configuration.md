# Engine Configuration

`engine.yaml` is the sole configuration surface for an engine — no code
changes needed to add a new variant or adjust parameter handling.
For the crate structure and how to build a custom engine binary,
see [Engine Internals](/dev/engine).

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

The engine registers its heartbeat in Valkey and appears in GET /service-info.
Submit runs using a `workflow_engine` that matches `engine.name`. For custom
parsing or log handling, implement the Engine trait and build a custom binary.
See [Engine Internals → Adding a Custom Engine](/dev/engine#adding-a-custom-engine).
