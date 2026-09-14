# IPC Protocol

Transport: Windows Named Pipe between Desktop and Core.

## Envelope (versioned request/response)

Fields (conceptual):

- `protocol_version` — negotiation / compatibility
- `request_id` — correlation
- `operation` — RPC method name
- `payload` — operation-specific body
- `status` / `error` — outcome
- correlation / tracing metadata

## Compatibility rules

- Prefer **additive** changes (new fields, new operations).
- Breaking changes require a `protocol_version` bump and negotiation support.
- Core remains the source of truth; Desktop must not assume silent schema drift.
