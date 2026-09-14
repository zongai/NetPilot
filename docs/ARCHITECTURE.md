# Architecture

```
Desktop.exe (WinUI 3)
        │  Named Pipe / versioned RPC
        ▼
Core.exe (Rust)     ← single source of truth
```

## Ownership

| Layer | Owns |
|-------|------|
| Desktop | UI shell, navigation, presentation of Core state |
| Core | Routing, proxy lifecycle, TUN, DNS, process attribution, connection state, diagnostics |
| Protocol adapters | Protocol-specific encode/decode and session logic |
| Transport adapters | TCP/UDP/TLS/WebSocket/HTTP2/gRPC/REALITY framing |

Protocol and transport are independent layers. REALITY is a security/transport capability, not a standalone protocol.

## Adapter lifecycle

`Create → Validate → Start → HealthCheck → Running → Reload/Restart → Stop`

## Privileged operations

TUN / WFP / WinDivert and other privileged operations require threat modeling, timeout, cancellation, rollback, tests, and human review.
