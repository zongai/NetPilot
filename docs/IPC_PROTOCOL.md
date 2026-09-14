# IPC Protocol

Transport: Windows Named Pipe between Desktop and Core.

## Envelope (versioned request/response/event)

Implemented in `netpilot-ipc` as `IpcEnvelope` (NP-015). Codec: JSON.

| Field | Role |
|-------|------|
| `protocol_version` | Negotiation / compatibility (`PROTOCOL_VERSION = 1`) |
| `kind` | `request` \| `response` \| `event` |
| `request_id` | Correlation between request and response |
| `operation` | RPC method name |
| `status` | `ok` \| `error` (responses) |
| `error` | `{ kind, message, code? }` — taxonomy from `docs/ERRORS.md` |
| `payload` | Operation-specific JSON |
| `correlation_id` | Optional tracing join key |

## Named pipe server (NP-016)

| Type | Role |
|------|------|
| `PipeServerConfig` | `pipe_name` (default `\\.\pipe\netpilot-core`), `accept_timeout`, `max_instances` |
| `NamedPipeServer` | State: Created → Listening → Connected → ShuttingDown → Closed |
| `PipeConnection` | Accepted session skeleton (I/O in later tasks) |

API: `listen`, `accept` (timeout / cancel), `shutdown`.  
`new_test` enables an in-process client signal for unit tests without OS pipes.  
OS `CreateNamedPipe` bind is intentionally thin on Windows (state transition only) until a dedicated transport wiring task; non-Windows requires `test_mode`.

## Compatibility rules

- Prefer **additive** changes (new fields, new operations).
- Breaking changes require a `protocol_version` bump and negotiation support.
- Core remains the source of truth; Desktop must not assume silent schema drift.
- Unsupported versions are rejected at decode (`EnvelopeError::UnsupportedVersion`).

## Security

- Never put passwords, tokens, or private keys in `payload` fields that are logged.
- Error `message` must stay free of secrets (`docs/SECURITY.md`).
- Pipe path is a local endpoint name only — not a credential.
