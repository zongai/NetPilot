# V5 Agent Rules
- Inspect V4 + S11 before changing code.
- Execute tasks in DAG order.
- UI is control plane; Core owns network/runtime state.
- Preserve versioned IPC/config contracts.
- Network operations need timeout/cancellation/error handling.
- Never log credentials or secrets.
- TUN/Wintun/driver/admin work requires human review.
- Prefer mature/auditable implementations for complex protocols.
- GUI launch is never sufficient acceptance; run tests.
