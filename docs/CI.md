# Windows CI workflow baseline (NP-009)

## Goals

- Every push/PR to `main` runs format, check, test, clippy on **windows-latest**.
- Toolchain matches `rust-toolchain.toml` (1.98.1 + rustfmt + clippy).
- Foundation for later **exe artifact** builds (release workflow can extend this).

## Workflow

File: `.github/workflows/ci.yml`

Triggers: `push` and `pull_request` to `main`.

Jobs:

1. **ci** (windows-latest)
   - checkout
   - install rust via `dtolnay/rust-toolchain` reading `rust-toolchain.toml`
   - `cargo fmt --all -- --check`
   - `cargo check --workspace`
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`

## Local parity

```powershell
./scripts/ci.ps1
```

## Artifacts (future)

Release/exe packaging will be a separate workflow or job (formal build: README/CHANGELOG discipline per project policy). This baseline does not upload release binaries yet.

## Secrets

CI must not print secrets. Use GitHub Secrets for any future signing certificates; never echo them.
