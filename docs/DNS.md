# DNS layer (S6, NP-073…NP-080)

Crate: `netpilot-dns`.

| NP | API |
|----|-----|
| 073 | `DnsResolver` / `DnsQuery` / `DnsResponse` |
| 074 | `SystemResolver::probe` |
| 075 | `UdpDnsTransport` |
| 076 | `TcpDnsTransport` |
| 077 | `DoTTransport` |
| 078 | `DohTransport` |
| 079 | `DnsCache` |
| 080 | `FakeIpAllocator` (default `198.18.0.0/16`) |

`MockTransport` seeds unit tests. Real sockets / TLS / HTTPS clients are deferred to later network wiring.
