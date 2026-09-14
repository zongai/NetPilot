# Agent workflow (NP-003)

Operational detail for automated agents. Short rules live in `/AGENTS.md`.

## Task lifecycle

```
Select NP-### → Pre-read → Plan → Edit (allow-list only) → Accept → Report → Commit/Push
```

### 1. Select

- Prefer the lowest unfinished task whose dependencies are done (`docs/TASKS.md`, task front-matter `Dependency`).
- Confirm Stage (S0–S9) and Area match the intended work.

### 2. Pre-read

Always:

- `README.md`, `AGENTS.md`
- `docs/ARCHITECTURE.md`, `docs/IPC_PROTOCOL.md`, `docs/TOOLCHAIN.md`
- `docs/tasks/NP-###.md` (full file)

When the task touches a subsystem, also open the relevant crate stubs and prior task notes.

### 3. Plan

Write a brief plan covering:

| Item | Content |
|------|---------|
| Goal | One-sentence task goal |
| Files | Paths inside `allowed_paths` only |
| Interfaces | Types / APIs to introduce or keep stable |
| Tests | Unit / integration / negative cases from the task matrix |
| Risks | Secrets, privilege, IPC break, Windows-only |
| Out of scope | Explicit non-goals (later NP ids) |

### 4. Edit

- Touch only `allowed_paths`.
- Keep secrets out of trees and logs.
- For network/TUN/privilege: timeouts, cancellation, rollback paths.
- Do not drive-by format the entire workspace unless the task is about formatting policy.

### 5. Accept

Run the commands listed under **Completion Acceptance Commands** in the task file.
Report environment gaps (e.g. no rustup, non-Windows host) instead of inventing pass results.

### 6. Report

Required summary fields (see `AGENTS.md` §7): files, tests, risks, human-review.

### 7. Commit

- Message prefix: `NP-###: …`
- Push to the agreed remote when credentials and permissions allow.
- Do not rewrite published history without an explicit baseline reason.

## Cross-task rules

| Topic | Rule |
|-------|------|
| IPC | Additive first; version bump for breaks |
| Config / schema | Version and migrate; no silent drift |
| Privileged ops | Human review before treating as done |
| Desktop vs Core | UI presents Core state; Core owns truth |
| Protocols vs transports | Keep layers independent |

## Failure / blocker handling

If blocked (missing allow-list path, missing Windows, missing secret, unclear requirement):

1. Stop further speculative edits
2. State the blocker clearly
3. Ask the human for the minimum decision needed

Do not invent parallel task scope to “work around” a blocker.
