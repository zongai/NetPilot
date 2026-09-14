# TUN layer (S5, NP-061…NP-070)

Crate: `netpilot-tun`.

| NP | API |
|----|-----|
| 061 | `TunProvider` / `MockTunProvider` |
| 062 | `WintunFeasibility` spike (no DLL link in CI) |
| 063 | `TunDevice` lifecycle: Created→Configured→Running→Stopped |
| 064 | `configure_ip` virtual interface address |
| 065 | `RouteManager` install/remove / full-tunnel helper |
| 066–067 | `PacketPipeline` ingress/egress queues |
| 068–069 | `TcpIntercept` / `UdpIntercept` |
| 070 | `BypassPolicy` loopback / link-local / private |

Real Wintun FFI and privileged route application are **out of scope** for these tasks; mocks keep CI deterministic on `windows-latest` without driver install.

