# Agent Rules

1. Read `README.md`, `docs/ARCHITECTURE.md`, `docs/IPC_PROTOCOL.md`, and the **target task** (`docs/tasks/NP-###.md`) before any edit.
2. Plan first. Implement **only** the current task. Do not expand into later tasks.
3. Modify **only** the task’s `allowed_paths`. Paths outside the allow-list are out of scope.
4. Establish stable types/interfaces before wiring behavior.
5. Network / TUN / driver / privilege work requires: timeout, cancellation, rollback, tests, and human review.
6. Never log passwords, tokens, private keys, or full proxy credentials. Redact secrets in logs and diagnostics.
7. Preserve versioned IPC and public API compatibility; prefer additive changes.
8. Run the task’s acceptance commands (`cargo fmt` / `check` / `test` / `clippy`) before declaring done. Use the pinned toolchain in `rust-toolchain.toml` (see `docs/TOOLCHAIN.md`).
9. Summarize changed files, tests, risks, and human-review needs when finishing a task.
