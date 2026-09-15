# R1 NP-257 — Network/thread/resource leak test plan

## Procedure (Windows)

1. Start Core, connect Desktop, idle 5 minutes.
2. Process Explorer / Resource Monitor: handle count, thread count, private bytes.
3. Add subscription + select proxy + generate light traffic 10 minutes.
4. Disconnect, `runtime.shutdown` or exit Desktop then Core.
5. Confirm no orphan `netpilot-core.exe`; named pipe released.

## Pass criteria

- Thread/handle counts stable within ±20% after idle settle
- No unbounded growth across 3 connect/disconnect cycles
- Pipe `\\.\pipe\netpilot-core` gone after Core exit

## Status

Not executed in this environment (no Windows agent). **Gate open until human/CI Windows run.**
