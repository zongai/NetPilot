# Error taxonomy and propagation conventions (NP-006)

## Goals

- Stable, readable error categories across Core, IPC, and Desktop.
- Safe propagation: no secret leakage; enough context for diagnosis.
- Map internal errors to IPC/user-facing codes without leaking internals.

## Taxonomy (stable names)

| Kind | When | Examples | Retry? |
|------|------|----------|--------|
| `InvalidInput` | Caller violated schema/precondition | bad URI, unknown field, failed validation | No (fix input) |
| `NotFound` | Missing entity | unknown proxy id, missing config profile | No |
| `Conflict` | State disagreement | already running, version mismatch | Maybe after reconcile |
| `Unavailable` | Dependency down | pipe disconnected, DNS upstream timeout | Yes (backoff) |
| `Timeout` | Deadline exceeded | IPC wait, health check, TUN setup | Yes (bounded) |
| `Cancelled` | Explicit cancel | user stop, shutdown, token cancel | No |
| `PermissionDenied` | Authz / elevation | IPC peer not allowed, need admin | No (elevate/consent) |
| `FailedPrecondition` | Wrong lifecycle state | stop before start, apply while shutting down | After state change |
| `Internal` | Bug or unexpected | invariant broken, unexpected OS error | Maybe once; log |
| `Unimplemented` | Stub / future task | skeleton protocol adapter | No |

These names are **logical**. Rust crates will map to `thiserror`/custom enums in implementation tasks; IPC will map to stable string/integer codes (NP-020).

## Propagation rules

1. **Add context at boundaries**, not in every leaf:
   - Leaf: low-level OS/`std` error
   - Boundary (IPC, adapter start, config apply): attach operation name + resource id (never secrets)
2. **Prefer typed errors** over stringly `anyhow` at public crate boundaries once implemented.
3. **Do not discard cancellation.** Map `Cancelled` through IPC so Desktop can stop spinners cleanly.
4. **Timeouts are first-class.** Network, pipe, and privileged ops must accept a deadline or `CancellationToken`-style handle.
5. **User-facing messages** are separate from log messages:
   - Log: detailed, redacted, English, structured fields
   - User: short, localized later, no stack traces by default

## IPC mapping (preview)

| Internal kind | IPC status class | Notes |
|---------------|------------------|--------|
| InvalidInput / NotFound / Conflict / FailedPrecondition | `4xx`-style application errors | Stable `error.code` |
| PermissionDenied | application + authz | Do not reveal peer identity details beyond need |
| Timeout / Unavailable / Cancelled | transient | Client may retry per policy |
| Internal / Unimplemented | `5xx`-style | Log id / correlation id only to user |

Envelope already carries `status` / `error` (`docs/IPC_PROTOCOL.md`). Concrete codes land with NP-015–NP-020.

## Logging correlation

Every error path that crosses IPC should preserve or allocate a **correlation / request id** so Desktop, Core logs, and diagnostics can join events.

## Anti-patterns

- `unwrap`/`expect` on production IPC and TUN paths
- Returning raw `Windows` error text that embeds paths with credentials
- Swallowing errors in background tasks without metrics/log
- Using `Internal` for validation failures (use `InvalidInput`)

## Implementation note

This document is the **contract**. Skeleton crates do not yet implement the enums; later tasks (`crates/core`, `crates/ipc`, …) must follow these names when introducing public error types.
