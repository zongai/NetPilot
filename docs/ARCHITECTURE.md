# Architecture and ownership boundaries (NP-004)

## System diagram

```
Desktop.exe (WinUI 3)
        │  Named Pipe / versioned RPC (docs/IPC_PROTOCOL.md)
        ▼
Core.exe (Rust)                    ← single source of truth
  ├─ crates/core          runtime orchestration
  ├─ crates/ipc           pipe server, envelope, routing
  ├─ crates/config        schema, load, validate, apply
  ├─ crates/rules         rule model, matchers
  ├─ crates/routing       decision engine
  ├─ crates/proxy         proxy profile & lifecycle
  ├─ crates/protocols/*   protocol adapters
  ├─ crates/transports/*  transport adapters
  ├─ crates/dns           resolver, Fake-IP, leak prevention
  ├─ crates/tun           TUN device & packet path
  ├─ crates/process       process discovery & attribution
  ├─ crates/diagnostics   probes, reports, connection inspect
  └─ crates/testkit       shared fixtures
```

## Ownership matrix

| Component | Owns | Must not own |
|-----------|------|--------------|
| **Desktop** (`apps/desktop`) | UI shell, navigation, binding to Core-exported state, user input | Routing decisions, TUN, DNS truth, credential storage logic |
| **Core binary** (`apps/core`) | Process entry, wiring subsystems, IPC listen loop | UI rendering |
| **crates/core** | Runtime state machine, startup/shutdown orchestration | Protocol wire formats |
| **crates/ipc** | Named pipe transport, versioned envelope, request router, events | Business policy (routing rules, proxy selection) |
| **crates/config** | Schema, load/normalize/validate/migrate, atomic apply | Live packet path |
| **crates/rules** | Rule AST, matchers (domain/IP/port/process hooks) | Proxy I/O |
| **crates/routing** | Decision engine, explain, priority | TUN device lifecycle |
| **crates/proxy** | Profile model, groups, health, lifecycle manager | UI lists |
| **crates/protocols/\*** | Encode/decode, session for one protocol family | Transport framing (except what protocol requires) |
| **crates/transports/\*** | TCP/UDP/TLS/WS/H2/gRPC/REALITY framing | Protocol identity / URI parse |
| **crates/dns** | Resolver abstraction, DoT/DoH, cache, Fake-IP, leak policy | System-wide routing table edits outside DNS path |
| **crates/tun** | Device lifecycle, IP config, ingress/egress, intercept | UI; long-term rule storage |
| **crates/process** | Discovery, stable identity, path/PID attribution | Rule evaluation engine |
| **crates/diagnostics** | Connection model, probes, reports, script boundaries | Proxy handshake crypto |
| **crates/testkit** | Fixtures, helpers for conformance tests | Production runtime paths |

## Layer rules

1. **Core is the source of truth.** Desktop displays and commands; it does not keep a divergent authority copy of routes, connections, or DNS state.
2. **Protocol ≠ transport.** Adapters compose: e.g. VLESS + REALITY is protocol + transport capability, not a single fused crate ownership.
3. **REALITY** is a security/transport capability under `crates/transports/reality`, not a standalone protocol crate.
4. **Privileged path isolation.** TUN / WFP / WinDivert / service install live behind explicit APIs with timeout, cancellation, rollback, and human review before “done”.
5. **IPC is the only Desktop↔Core control plane** for production. Ad-hoc files or shared memory for control are out of scope unless a future task promotes them.

## Adapter lifecycle (proxy / protocol / transport)

```
Create → Validate → Start → HealthCheck → Running ⇄ Reload/Restart → Stop
```

Failures at Validate/Start must leave no leaked devices, routes, or elevated handles.

## Crate boundary checklist (for later NP tasks)

When adding code to a crate:

- Depend **down** the ownership matrix, not sideways into UI or unrelated adapters without an interface.
- Public API changes that cross IPC need versioning (`docs/IPC_PROTOCOL.md`).
- Secrets never appear in `Display`/`Debug` without redaction (`docs/SECURITY.md` once NP-005 lands).

## Non-goals (this stage)

- Implementing TUN or protocol wire formats (later stages)
- Shipping a full WinUI feature set (S4)
- Third-party plugin loading without sandbox limits (S8)
