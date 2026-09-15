# NetPilot V5 Functional Implementation Task Pack

This is an **implementation task pack**, not a completed executable.

It is specifically designed to turn the current V4/S11 GUI skeleton into a functioning client. The intended end-to-end path is:

Desktop → Named Pipe IPC → Core → Subscription/Proxy → Rules/Routing → Connections/Logs → Desktop

Acceptance flow:
1. Core becomes Ready.
2. Desktop connects to Core over Named Pipe.
3. Add/update subscription.
4. Parse nodes into unified ProxyProfile.
5. Nodes appear in Proxies.
6. Connect/disconnect proxy.
7. Rules produce deterministic routing decisions.
8. Connections and logs become observable.
9. Diagnostics work.
10. Windows acceptance test passes.

Exactly 96 tasks: NP-145 through NP-240, 8 stages × 12.
