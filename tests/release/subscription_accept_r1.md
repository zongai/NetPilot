# R1 NP-259 — Subscription release acceptance

## Cases

1. `subscription.add` with HTTPS URL (real-http build)
2. `subscription.update` parses nodes → `proxy.list` non-empty
3. Invalid URL returns structured error (no panic)
4. Logs redaction: token query not plaintext

## Status

Unit/integration coverage exists under `netpilot-subscription` and Core IPC handlers.
End-to-end against public network: **human/CI Windows**.
