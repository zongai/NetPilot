# Toolchain and formatting policy (NP-002)

## Pinned toolchain

| Item | Value |
|------|--------|
| Channel / version | **1.98.1** (stable) |
| Components | `rustfmt`, `clippy` |
| Profile | `minimal` |
| File | `rust-toolchain.toml` |
| MSRV (`rust-version`) | 1.98.1 (`Cargo.toml` `[workspace.package]`) |

Do **not** use the floating `stable` alias for CI or release builds. Bump the pin deliberately (document in CHANGELOG when doing formal builds).

Install / switch (with rustup):

```powershell
rustup show
# Should report 1.98.1 when in this repo
cargo --version
rustfmt --version
cargo clippy --version
```

## rustfmt policy

Use stock `rustfmt` defaults for 1.98 unless a root `rustfmt.toml` is added in a later task. Required checks:

```powershell
cargo fmt --all -- --check
```

Conventions (must match fmt output):

- Edition 2021 formatting
- Standard max width / import grouping as emitted by this toolchain’s rustfmt
- No manual fight against rustfmt; reformat with `cargo fmt --all` before review

## clippy policy

CI and `scripts/ci.ps1` run:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
```

Workspace lint table (`Cargo.toml` `[workspace.lints.*]`):

- `unsafe_code` = warn (prefer safe APIs; justify `unsafe` when needed)
- `clippy::all` = warn
- `clippy::pedantic` / `nursery` = allow for now (tighten in later S0/S1 tasks)

Member crates should add the following when they gain real code (not required for pure stubs):

```toml
[lints]
workspace = true
```

## Acceptance commands (all platforms with the pinned toolchain)

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Or: `./scripts/ci.ps1`

## Bumping the pin

1. Update `rust-toolchain.toml` `channel`
2. Update `Cargo.toml` `workspace.package.rust-version`
3. Update this document and README “Development” section
4. Run full acceptance commands on Windows CI before merge
