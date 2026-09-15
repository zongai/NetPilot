# R1 NP-253 — Windows permission / UAC flow

## Operations requiring elevation

- Creating Wintun adapter / TUN start
- System route injection / rollback
- System proxy set/disable (may prompt depending on policy)

## App behavior

1. Desktop runs as standard user by default.
2. Core starts without admin for IPC, subscription, SOCKS inbound on high ports if permitted.
3. Privileged ops return structured IPC errors when denied; UI surfaces message — no silent fail loops.
4. Manifest: Desktop/Core are **not** `requireAdministrator` by default (avoid permanent elevation).

## Human review

Shipping a permanently elevated Core requires explicit product decision and code signing.
