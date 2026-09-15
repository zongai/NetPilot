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
