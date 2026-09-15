# R1 NP-254 — Release logging / crash diagnostics

## Logging

- Core: stderr structured lines via `log_line`; level from `NETPILOT_LOG_LEVEL`
- Data dir logs folder: `%LOCALAPPDATA%\NetPilot\data\logs` (created on first-run)
- Secrets redacted before emission

## Crash handling

- Desktop WPF: unhandled exceptions should surface MessageBox (existing pattern)
- Core: process exit codes — `1` lifecycle failure, `2` instance lock conflict
- No automatic crash dump upload (privacy); local dumps optional via Windows Error Reporting

## Operator steps

1. Reproduce with `NETPILOT_LOG_LEVEL=debug`
2. Capture `logs.list` IPC and stderr
3. Attach VERSION.txt + OS build string
