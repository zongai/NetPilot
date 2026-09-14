# Test conventions and fixtures (NP-008)

## Goals

- Deterministic unit tests by default.
- Clear separation: unit / integration / Windows-only / conformance.
- Fixtures live in `crates/testkit` as the workspace grows.

## Categories

| Kind | Where | Notes |
|------|-------|-------|
| Unit | `src/*.rs` `#[cfg(test)]` or `tests/` | No network, no admin, no real TUN |
| Integration | `tests/` per crate or Core | May use localhost loops; still no privileged device unless marked |
| Conformance | `crates/testkit` + protocol fixtures | Golden vectors for parsers/rules |
| Windows smoke | `scripts/smoke.ps1` + future harnesses | TUN/process/desktop; human or CI windows-2022 |

## Rules

1. **Determinism:** no wall-clock flakiness; inject clocks/timeouts where needed.
2. **Secrets:** fake credentials only; do not print them in assertion messages when avoidable.
3. **Timeouts:** async tests must bound waits; prefer cancellation tests for IPC/network.
4. **Naming:** `feature_scenario_expected()` style; include negative cases from task matrices.
5. **Workspace:** `cargo test --workspace` must pass on non-Windows hosts for non-`#[cfg(windows)]` tests.

## Fixtures

- Prefer shared builders in `netpilot-testkit` over copy-pasted JSON.
- Rule/DNS/protocol vectors should be data-driven where practical.

## CI expectation

- PR CI runs unit/integration on Windows (see `docs/CI.md`).
- Privileged tests are opt-in / nightly or manual until harnesses exist.
