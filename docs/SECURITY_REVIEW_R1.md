# R1 NP-255 — Release security review

Status: **review document complete**; gates must be re-validated on each RC binary.

## Threat model (local Windows client)

- Malicious local process attaching to Named Pipe
- Config / subscription URL leakage in logs
- Privilege escalation via TUN / system proxy
- Supply-chain: third-party `wintun.dll`

## Controls present in tree

| Area | Control |
|------|---------|
| IPC | Local named pipe; auth policy + privilege map; payload redaction helpers |
| Logs | `redact_secrets` / ConnectionLog redaction; no password fields in summaries |
| Config | Secrets in ProxyProfile skipped in redacted maps |
| Instance | Single-instance lock under run dir |
| TUN | Probe + fail-safe; admin required for adapter (human review) |
| System proxy | Explicit IPC ops; disable path available |

## Must-not-ship

- `NETPILOT_SMOKE_ONLY=1` in production shortcuts
- Test credentials or hardcoded subscription tokens
- Debug endpoints not in the documented operation list
- Unstripped binaries with embedded secrets

## Residual risks (accepted for RC)

- Named pipe peer identity is best-effort without full Windows SDDL lockdown (follow-up hardening)
- REALITY / VMess wire compatibility is subset of commercial clients
- Wintun DLL must be obtained from trusted upstream and hash-pinned by human releaser

## Sign-off

- Automated: `scripts/secret-scan-r1.ps1`
- Human: required before NP-264 Final
