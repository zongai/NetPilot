# NetPilot Desktop (WinUI 3)

S4 UI shell. Network interception remains in **Core**; Desktop uses named-pipe IPC.

## Layout

| Path | Role |
|------|------|
| `App.xaml(.cs)` | Application entry (NP-049) |
| `MainWindow.xaml(.cs)` | Shell + nav host |
| `Navigation/AppPage.cs` | Page tags / smoke list |
| `ViewModels/` | Runtime, Proxies, Rules, Connections, Logs, Diagnostics |
| `Services/IpcService.cs` | `IIpcService` + loopback (NP-059) |
| `Models/UserError.cs` | User-facing errors (NP-058) |

## Task map

| NP | Goal |
|----|------|
| 051 | Runtime state binding |
| 052 | Proxy node/group list |
| 053 | Rule list |
| 054–055 | Connection list + detail |
| 056 | Log stream (redacts secret-like lines) |
| 057 | Diagnostics |
| 058 | `UserError` model |
| 059 | IPC service abstraction |
| 060 | Smoke tags in `AppPageMap.SmokeTags` |

## Smoke baseline (NP-060)

1. App launches and shows MainWindow
2. Navigate every tag in `AppPageMap.SmokeTags`
3. Home shows runtime state after `health.check` (loopback OK)
4. Proxies/Rules/Connections sample lists non-empty
5. Logs refuse lines containing `password=` / `uuid=`
6. Diagnostics runs without throwing

Rust CI does **not** build WinUI; structure is versioned for review.
