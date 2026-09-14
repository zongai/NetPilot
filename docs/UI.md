# Desktop UI (S4)

WinUI 3 shell under `apps/desktop/`. Core remains the source of truth for policy and TUN.

## IPC from UI

- Abstraction: `IIpcService` (`apps/desktop/Services/IpcService.cs`)
- Envelope mirror: `IpcEnvelopeDto` (aligned with `crates/ipc` JSON fields)
- Loopback service used until OS pipe client is wired
- Operations used by shell: `health.check`, `health.ready`, `runtime.state`

## Error surface

`UserError` / `UserErrorKind` map IPC `ErrorBody.kind` to UI-safe messages. Never place passwords or tokens in `Message`.

## Pages

`home` · `proxies` · `rules` · `connections` · `logs` · `diagnostics` · `settings`
