# NetPilot Desktop (WinUI 3)

Shell skeleton for S4 UI tasks.

| Task | Content |
|------|---------|
| NP-049 | `App.xaml` / `MainWindow` application shell |
| NP-050 | `NavigationView` + `AppPage` page model |

Network interception and policy remain in **Core** (`apps/core` + crates). Desktop talks to Core via named-pipe IPC (`crates/ipc`).

Build/packaging (MSBuild / Windows App SDK) is outside the Rust CI workspace; these sources are tracked for structure and review.
