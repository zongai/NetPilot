# Bootstrap E2E (NP-156)

Linux/CI: `NETPILOT_SMOKE_ONLY=1 cargo run -p netpilot-core`

Windows: start `netpilot-core.exe`, confirm Named Pipe `\\.\pipe\netpilot-core` accepts `ping`.

Unit coverage lives in `netpilot-core-lib` (health, shutdown, instance, logging) and `netpilot-config` (`RuntimePaths`).
