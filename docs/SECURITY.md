# Security baseline and secret-redaction policy (NP-005)

## Goals

- Prevent credential and key leakage via logs, diagnostics, IPC traces, test output, or commits.
- Keep privileged operations (TUN, WFP, services) explicit, time-bounded, and reviewable.
- Prefer least privilege for Core and Desktop processes.

## Secret classes

| Class | Examples | Handling |
|-------|----------|----------|
| **Credentials** | Proxy passwords, UUID/user id used as auth, API tokens | Never log in full; redact in `Debug`/`Display`; store via dedicated secret boundary (later NP-035) |
| **Keys** | Private keys, REALITY shortIds paired with secrets, TLS client keys | Memory-only where possible; no plaintext in config dumps |
| **Material** | Subscription tokens, cookie headers, Authorization values | Strip or mask in connection inspector exports |
| **PII-ish** | Full process command lines with embedded secrets | Prefer path + hash identity over raw command line in logs |

## Redaction rules

1. **Default deny for secrets in logs.** Structured fields use names like `password_redacted: true` rather than values.
2. **Masking format:** show at most prefix/suffix with fixed mask, e.g. `u****@ex.com`, `sk-****abcd` (never enough to reconstruct).
3. **Errors:** user-facing and log errors must not echo passwords or full URIs with userinfo (`scheme://user:pass@host`).
4. **IPC:** payloads that intentionally carry secrets for setup must not be mirrored to diagnostic event streams.
5. **Tests:** fixtures use obviously fake values (`test-password`, `00000000-0000-0000-0000-000000000000`); still avoid printing them in assert messages when easy.
6. **Core dumps / panic:** avoid putting secrets in types that are included in panic payloads; prefer `Redacted` wrappers.

## Recommended type pattern (for later implementation tasks)

```text
struct SecretString { /* redacted in Debug/Display */ }
```

Public APIs that accept secrets should take a dedicated type, not bare `String`, once those crates leave skeleton stage.

## Privileged operations

| Operation | Requirements |
|-----------|----------------|
| TUN create/destroy | timeout, cancellation, rollback, tests, human review |
| Route install/remove | same; document failure partial state |
| WFP / WinDivert | threat model note in task report; no silent permanent filters |
| Service install | explicit user consent path (Desktop); Core does not self-elevate without policy |

## IPC and local trust

- Named pipe is **local machine** trust: still apply auth boundary (later NP-022).
- Do not treat “localhost” as proof of end-user identity alone.
- Reject oversized payloads; avoid unbounded allocations on untrusted input.

## Repository hygiene

- No secrets in git history, issues, or CI logs.
- `GITHUB_TOKEN` and similar live outside the repo (agent: secrets file only; never in `AGENTS` memory body).
- Scan PR diffs for accidental credential patterns before merge when practical.

## Incident response (lightweight)

If a secret is committed or logged:

1. Rotate the credential immediately.
2. Purge or rewrite history only with explicit human approval.
3. Document in CHANGELOG for formal releases if user impact is possible.

## Related docs

- `AGENTS.md` — agent must never log secrets
- `docs/ARCHITECTURE.md` — privileged path isolation
- `docs/IPC_PROTOCOL.md` — versioned control plane
