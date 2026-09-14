# Documentation index (NP-011)

## Start here

| Doc | Purpose |
|-----|---------|
| [../README.md](../README.md) | Project overview, layout, dev commands |
| [../AGENTS.md](../AGENTS.md) | Agent operating rules (mandatory for automation) |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Ownership boundaries and crate matrix |
| [IPC_PROTOCOL.md](IPC_PROTOCOL.md) | Named pipe envelope and compatibility |
| [TOOLCHAIN.md](TOOLCHAIN.md) | Rust 1.98.1 pin, fmt/clippy policy |
| [SECURITY.md](SECURITY.md) | Secrets, redaction, privilege ops |
| [ERRORS.md](ERRORS.md) | Error taxonomy and propagation |
| [LOGGING.md](LOGGING.md) | Structured logging baseline |
| [TESTING.md](TESTING.md) | Test conventions and fixtures |
| [CI.md](CI.md) | Windows GitHub Actions baseline |
| [RELEASE_GATES.md](RELEASE_GATES.md) | Release checklist |
| [AGENT_WORKFLOW.md](AGENT_WORKFLOW.md) | Agent task lifecycle detail |
| [TASKS.md](TASKS.md) | Full NP-001…NP-120 index |
| [tasks/](tasks/) | Per-task specifications |

## Stage map

| Stage | Focus | Tasks |
|-------|--------|-------|
| S0 | Foundation (docs, toolchain, CI) | NP-001 … NP-012 |
| S1 | IPC / Core runtime | NP-013 … NP-024 |
| S2 | Config & proxy lifecycle | NP-025 … NP-036 |
| S3 | Rules & routing | NP-037 … NP-048 |
| S4 | Desktop UI | NP-049 … NP-060 |
| S5 | TUN | NP-061 … NP-072 |
| S6 | DNS | NP-073 … NP-084 |
| S7 | Process & RuleSet | NP-085 … NP-096 |
| S8 | Connections & diagnostics | NP-097 … NP-108 |
| S9 | Protocols & transports | NP-109 … NP-120 |
