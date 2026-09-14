# Structured logging and redaction baseline (NP-007)

## Goals

- Structured, correlatable logs for Core and (later) Desktop-side services.
- Mandatory secret redaction (`docs/SECURITY.md`).
- Compatible with local debug and CI capture without flooding.

## Levels

| Level | Use |
|-------|-----|
| `ERROR` | Operation failed; includes error kind (`docs/ERRORS.md`) + correlation id |
| `WARN` | Recoverable degradation, retries, deprecated paths |
| `INFO` | Lifecycle: start/stop, config apply, adapter state transitions |
| `DEBUG` | Detailed decision traces (route explain, handshake steps) — no secrets |
| `TRACE` | Packet-level / high volume — off by default in release |

## Required fields (when applicable)

- `ts` — timestamp (UTC)
- `level`
- `target` — module / crate
- `correlation_id` / `request_id` — join IPC and subsystem logs
- `event` — stable event name (`core.start`, `proxy.health_fail`, …)
- `error.kind` — taxonomy name when logging failures

## Redaction

- Apply `docs/SECURITY.md` before emit.
- Never log full proxy URIs with userinfo.
- `Debug` implementations on secret-bearing types must redact.

## Implementation direction (later crates)

- Prefer a single facaded logger in Core (e.g. `tracing` + JSON or key=value layer).
- Desktop may use platform logging but should forward correlation ids on IPC calls.

## Anti-patterns

- `println!` for production paths
- Logging entire config blobs
- ERROR without `error.kind` or correlation when on IPC path
