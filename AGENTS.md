# Agent Rules (NP-003)

Canonical operating rules for Codex and other automated agents working on NetPilot.
Humans should follow the same discipline; see also `CONTRIBUTING.md`.

## 0. Mandatory pre-read

Before any edit:

1. `README.md`
2. `docs/ARCHITECTURE.md`
3. `docs/IPC_PROTOCOL.md`
4. `docs/TOOLCHAIN.md`
5. `docs/SECURITY.md`
6. The **target task** only: `docs/tasks/NP-###.md`
7. This file (`AGENTS.md`)

Optional when relevant: `docs/TASKS.md`, `docs/AGENT_WORKFLOW.md`, `docs/ERRORS.md`, `docs/SECURITY.md`.

## 1. One task at a time

- Implement **only** the current `NP-###`.
- Do **not** expand into later tasks, even if dependencies look trivial.
- Do **not** “clean up” unrelated crates, docs, or CI while finishing a task.

## 2. Path allow-list

- Modify **only** paths listed in the task’s `allowed_paths`.
- Anything outside the allow-list is out of scope for that task.
- If a required change is outside the allow-list, stop and report the blocker; do not widen scope silently.

## 3. Plan before edit

- State a short plan (goal, files, tests, risks) before writing code.
- Establish stable types / interfaces before wiring behavior.
- Prefer small, reviewable diffs.

## 4. Safety and secrets

- **Never** log or commit passwords, tokens, private keys, or full proxy credentials.
- Redact secrets in logs, diagnostics, error messages, and test fixtures.
- Do not embed live credentials in source, docs, or CI logs.
- Full policy: `docs/SECURITY.md`.
- Network / TUN / driver / privilege work requires: **timeout**, **cancellation**, **rollback**, tests, and **human review**.

## 5. Compatibility

- Preserve versioned IPC and public API compatibility.
- Prefer additive changes; breaking changes need an explicit version bump and negotiation support (`docs/IPC_PROTOCOL.md`).
- Core is the single source of truth; Desktop must not invent parallel state.

## 6. Toolchain and acceptance

- Use the pinned toolchain in `rust-toolchain.toml` (see `docs/TOOLCHAIN.md`).
- Before declaring a task done, run the task’s acceptance commands (typically):

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

- Or `./scripts/ci.ps1` when available on a full toolchain host.
- Sandbox limitations (missing rustfmt/clippy, no Windows) must be reported honestly; do not claim green checks that did not run.

## 7. Finish report

When finishing a task, summarize:

1. Changed files (and why)
2. Tests run / not run and results
3. Risks and residual gaps
4. Human-review needs (especially TUN, process, privilege, desktop)

## 8. Git and version control

- Branch / commit messages should include `NP-###`.
- Agent-owned version control: keep history clear; one logical commit per completed task when practical.
- Do not force-push shared branches unless explicitly required to establish a baseline (document why).

## 9. What agents must not do

- Skip the pre-read or ignore `allowed_paths`
- Implement multiple NP tasks in one pass without instruction
- Weaken secret redaction or IPC versioning “temporarily”
- Claim Windows/TUN validation without actual Windows evidence
- Store secrets in memory documents or commit them to the repo
