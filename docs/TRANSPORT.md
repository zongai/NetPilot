# Transport wiring (TLS / REALITY)

## TLS (`netpilot-transport-tls`)

| Type | Role |
|------|------|
| `TlsClientConfig` | SNI, ALPN, cert verify mode, backend selection |
| `TlsClientSession` | Connect/close lifecycle |
| `TlsBackend::Mock` | Default in CI |
| `TlsBackend::Platform` / `Rustls` | Reserved; return `Unsupported` until linked |

## REALITY (`netpilot-transport-reality`)

`RealityConfig` wraps `RealityTlsOverlay` and opens a mock TLS session for tests.  
Public keys must stay redacted in logs (`public_key_redacted`).
