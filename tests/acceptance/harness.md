# Functional acceptance harness (NP-240)

Checklist (must be measured against a running Core):

1. Core reaches Ready (`runtime.state` / `health.check`)
2. Named Pipe connects from Desktop
3. `subscription.add` + `subscription.update` yields nodes
4. `proxy.list` shows profiles
5. `proxy.select` sets active outbound
6. `rules.load` + `rules.decide` returns deterministic outbound
7. `connections.list` observable after traffic
8. `logs.list` redacts secrets
9. Diagnostics (`netstack.stats`, `tun.wintun_probe`) respond
10. Windows package layout contains core + GUI

Automated subset: `cargo test --workspace` on CI.
