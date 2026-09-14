# S11 Airport Subscription (`netpilot-subscription`)

Pipeline:

```text
URL → Fetch (timeout/ETag) → Decode (Base64) → Parse (URI|Clash|Sing-box)
  → Normalize (ProxyProfile) → Dedup/Filter/Rename → Group → Apply
```

| NP | Module |
|----|--------|
| 121–128 | manager, profile, fetcher, http, cache, policy, rollback |
| 129–132 | decoder, parsers/{uri,clash,singbox} |
| 133–139 | normalizer, transport_map, fingerprint, filter, rename, groups, merge |
| 140–144 | metadata, validate, diagnose, desktop UI, e2e tests |

MockFetcher is the default test transport (no sockets in CI).
