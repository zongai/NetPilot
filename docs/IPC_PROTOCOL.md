# IPC Protocol

Transport: Windows Named Pipe between Desktop and Core.

## Envelope (NP-015)

`IpcEnvelope` JSON: `protocol_version`, `kind`, `request_id`, `operation`, `status`, `error`, `payload`, `correlation_id`.

## Named pipe (NP-016 / NP-017)

| Type | Role |
|------|------|
| `NamedPipeServer` | Core listen/accept/shutdown (+ `test_mode`) |
| `NamedPipeClient` | Desktop connect/close (+ `test_mode`) |
| Default name | `\\.\pipe\netpilot-core` |

## Routing & events (NP-018 / NP-019)

- `RequestRouter` maps `operation` → handler; unknown → `NotFound`
- `EventStream` bounded queue; drops oldest when full

## Errors / timeout / auth / negotiate / health (NP-020…024)

- `map_pipe_error` / `map_route_error` → `ErrorBody` + numeric codes
- `CancelToken`, `Deadline`, `check_budget`
- `LocalAuthPolicy` + `privilege_for_operation`
- `negotiate` version overlap; `health.check` / `health.ready`

## Compatibility

Additive changes preferred; version bumps for breaks. No secrets in logged payloads.

## rules.decide (NP-INTEGRATION-001)

**Operation:** `rules.decide`

**Canonical request payload** (object, required):

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `domain` | string | one of domain/ip | Host name for domain rules |
| `ip` | string | one of domain/ip | Literal IP for CIDR rules |
| `port` | number (u16) | no | Optional port |

Examples:

```json
{ "domain": "www.google.com", "port": 443 }
{ "ip": "10.0.0.1", "port": 80 }
```

**Errors (`status: error`, kind `invalid_input`, code 400):**

| Condition | `error.message` |
|-----------|-----------------|
| no payload field | `missing payload` |
| payload `{}` | `empty payload` |
| payload not object / wrong field types | `malformed payload` |
| neither domain nor ip | `payload.domain or payload.ip required` |

**Success payload:** `outbound`, `explanation`, `matcher`, echo `request`.

Related: `rules.load` requires `{ "text": "<rule lines>" }`.

## proxy.upsert / proxy.list (NP-INTEGRATION-002)

**Operation:** `proxy.upsert`

| Field | Type | Required |
|-------|------|----------|
| `id` | string | yes |
| `server` | string | yes |
| `port` | number (1–65535) | yes |
| `name` | string | no (defaults to id) |
| `protocol` | string | no (default `socks5`) |
| `password` / `uuid` / `username` / `sni` / `alpn` | string | no |

Success payload: `{ "id", "accepted": true, "count": <usize> }`.

**Operation:** `proxy.list` — no payload. Returns `{ "items": [...], "selected": ... }` from Core profile store.

## subscription.update (NP-INTEGRATION-003)

**Operation:** `subscription.update`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | string | yes | Must exist via `subscription.add` |
| `body` | string | no | If set, skip HTTP; use as subscription body (decode→parse) |
| `timeout_secs` | number | no | HTTP fetch timeout (default 30) |

Pipeline: body → detect/decode (plain/base64) → parse (URI/Clash/Sing-box) → normalize `ProxyProfile` → `TrafficEngine.add_profile` → visible in `proxy.list`.

Success: `{ id, bytes, nodes, node_count, http_mode }` where `http_mode` is `inline-body` | `real-http` | `mock`.
