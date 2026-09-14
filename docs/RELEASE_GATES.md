# Baseline release gates and review checklist (NP-012)

## When this applies

Formal builds, version tags, and any workflow that publishes **exe** or installers.

## Pre-build gates

- [ ] All acceptance commands green on Windows CI (`docs/CI.md`)
- [ ] No known critical security issues open (`docs/SECURITY.md`)
- [ ] IPC/API compatibility reviewed if envelope or public types changed
- [ ] Privileged/TUN/process changes have human review notes
- [ ] Secrets not present in tree, logs, or artifacts
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Version numbers updated (workspace package / release tag) as applicable

## Documentation gates (formal build policy)

- [ ] **README** reflects current architecture and how to run/build
- [ ] **CHANGELOG** summarizes changes **since the previous successful formal build**
- [ ] Toolchain pin documented if bumped (`docs/TOOLCHAIN.md`)

## Post-build gates

- [ ] GitHub Actions workflow completed successfully
- [ ] Artifacts smoke-checked (Core starts; Desktop optional per stage)
- [ ] Tag / release notes published when shipping externally

## Review checklist (PR / task)

| Check | Owner |
|-------|--------|
| Allow-list / scope | Author + reviewer |
| Tests or documented gap | Author |
| Error kinds follow `docs/ERRORS.md` | Reviewer |
| Logging redaction | Reviewer |
| Windows impact called out | Author |
| CODEOWNERS paths | Auto + teams |

## Agent responsibility

Agents own version control updates for completed tasks and must not mark a formal release “done” without the gates above.
