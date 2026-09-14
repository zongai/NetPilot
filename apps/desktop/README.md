# NetPilot Desktop (WinUI 3)

Full UI shell for Core: Home, Proxies, Rules, Connections, Logs, Diagnostics, **Subscriptions**, Settings.

Network interception stays in **Core**; Desktop uses named-pipe IPC (`LoopbackIpcService` until the real pipe client is wired).

## Build (Windows)

```powershell
# from apps/desktop
dotnet publish NetPilot.Desktop.csproj -c Release -r win-x64 --self-contained true -o ..\..\dist\desktop
```

Full product package (Core + GUI):

```powershell
./scripts/package-full.ps1
```

CI: `.github/workflows/release-full.yml` (tag `v*` or `workflow_dispatch`).

## Layout

| Path | Role |
|------|------|
| `NetPilot.Desktop.csproj` | Unpackaged WinUI 3 / .NET 8 project |
| `Program.cs` / `App.xaml` | Application entry |
| `MainWindow.xaml` | Shell + navigation |
| `SubscriptionPage.xaml` | S11 subscription UI |
| `ViewModels/` | Runtime, Proxies, Rules, Connections, Logs, Diagnostics, Subscriptions |
| `Services/IpcService.cs` | IPC abstraction + loopback |

## Smoke

1. App launches MainWindow  
2. Navigate all tags including `subscriptions`  
3. Home shows runtime via loopback `health.check`  
4. Subscription page lists sample rows  
