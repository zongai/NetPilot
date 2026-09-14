# IPC Protocol

Transport: Windows Named Pipe between Desktop and Core (pipe server/client in later NP tasks).

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

## Compatibility rules

- Prefer **additive** changes (new fields, new operations).
- Breaking changes require a `protocol_version` bump and negotiation support.
- Core remains the source of truth; Desktop must not assume silent schema drift.
- Unsupported versions are rejected at decode (`EnvelopeError::UnsupportedVersion`).

## Security

- Never put passwords, tokens, or private keys in `payload` fields that are logged.
- Error `message` must stay free of secrets (`docs/SECURITY.md`).
