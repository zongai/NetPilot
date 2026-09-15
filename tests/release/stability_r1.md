# R1 NP-258 — Long-running stability test plan

## Procedure

- Duration: ≥ 4 hours Core + Desktop connected
- Workload: periodic `health.check`, `proxy.list`, `rules.decide`, subscription refresh interval if configured
- Monitor: memory, CPU, event log

## Pass criteria

- No crash, no IPC hang > 30s
- Memory RSS does not climb linearly without bound

## Status

**Not executed here.** Gate open for Windows lab.
