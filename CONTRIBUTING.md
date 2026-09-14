# Contributing

## Toolchain

Pinned Rust **1.98.1** + `rustfmt` + `clippy` (see `rust-toolchain.toml` and `docs/TOOLCHAIN.md`).

```powershell
rustup show   # expect 1.98.1 inside this repo
```

## Branches and PRs

- Use the task id in branch and PR titles: `NP-###` (e.g. `NP-002-toolchain-pin`).
- One primary task per PR when possible.
- Reference `docs/tasks/NP-###.md` in the PR description.

## Before merge

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Or run `./scripts/ci.ps1`.

Formatting and lint policy details: `docs/TOOLCHAIN.md`.

## Windows-sensitive changes

TUN, process attribution, service, privilege, or desktop integration changes require:

- Windows smoke testing (`./scripts/smoke.ps1` plus scenario-specific checks)
- Explicit human review

## Scope discipline

Respect each task’s `allowed_paths`. Do not drive-by refactor unrelated crates or docs.
