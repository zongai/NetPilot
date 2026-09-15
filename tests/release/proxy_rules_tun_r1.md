# R1 NP-260 — Proxy / rules / TUN full acceptance

## Cases

1. `rules.load` + `rules.decide` deterministic outbound
2. `proxy.upsert` / `proxy.select` / `proxy.list`
3. `tunnel.status` / `tun.wintun_probe` fail-safe without DLL
4. With Wintun + admin: `tunnel.start` human-supervised only

## Status

Logic covered by unit tests; TUN elevated path **requires human review**.
