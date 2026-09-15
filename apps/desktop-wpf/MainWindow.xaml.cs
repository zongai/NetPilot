using System.Text;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;

namespace NetPilot.Desktop.Wpf;

public partial class MainWindow : Window
{
    private bool _ready;
    private readonly CoreIpcClient _ipc = new();
    private string _coreState = "disconnected";
    /// <summary>True only after a successful IPC handshake (pipe.IsConnected alone is unreliable).</summary>
    private bool _ipcReady;

    public MainWindow()
    {
        InitializeComponent();
        _ready = true;
        // Defer navigation until after Core handshake to avoid permanent "offline" body text.
        Loaded += async (_, _) =>
        {
            await TryConnectCoreAsync();
            if (NavList.Items.Count > 0 && NavList.SelectedIndex < 0)
                NavList.SelectedIndex = 0;
            else if (NavList.SelectedItem is ListBoxItem { Tag: string tag })
                await ShowPageAsync(tag);
            else
                await ShowPageAsync("home");
        };
        Closed += (_, _) => _ipc.Dispose();
    }

    private async Task TryConnectCoreAsync()
    {
        _ipcReady = false;
        Exception? last = null;
        StatusText.Text = "Connecting to Core… (pipe)";

        for (var attempt = 1; attempt <= 3; attempt++)
        {
            try
            {
                if (!_ipc.IsConnected)
                {
                    try
                    {
                        await _ipc.ConnectAsync(attempt == 1 ? 2500 : 5000);
                    }
                    catch when (attempt == 1)
                    {
                        TryLaunchCore();
                        await Task.Delay(1200);
                        await _ipc.ConnectAsync(6000);
                    }
                }

                StatusText.Text = "Handshaking with Core…";
                // Prefer simple ping; fall back to health.check / runtime.state.
                JsonElement resp;
                try
                {
                    resp = await _ipc.RequestAsync("ping");
                }
                catch (Exception pingEx)
                {
                    StatusText.Text = $"ping failed, try health… ({pingEx.Message})";
                    resp = await _ipc.RequestAsync("health.check");
                }

                _ipcReady = true;
                _coreState = "connected";
                if (resp.TryGetProperty("payload", out var payload))
                {
                    if (payload.TryGetProperty("runtime_state", out var st))
                        _coreState = st.GetString() ?? "connected";
                    else if (payload.TryGetProperty("pong", out _))
                        _coreState = "running";
                    else if (payload.TryGetProperty("state", out var st2))
                        _coreState = st2.GetString() ?? "connected";
                }

                StatusText.Text = $"Core: {_coreState} (IPC ok)";
                return;
            }
            catch (Exception ex)
            {
                last = ex;
                _ipcReady = false;
                try { _ipc.Disconnect(); } catch { /* ignore */ }
                await Task.Delay(400 * attempt);
            }
        }

        _coreState = "disconnected";
        StatusText.Text = last is null
            ? "Core offline (start netpilot-core.exe)"
            : $"Core offline: {last.Message}";
        System.Diagnostics.Debug.WriteLine(last);
    }

    /// <summary>NP-146: launch netpilot-core.exe next to this GUI when offline.</summary>
    private static void TryLaunchCore()
    {
        try
        {
            var baseDir = AppContext.BaseDirectory;
            var candidates = new[]
            {
                System.IO.Path.Combine(baseDir, "netpilot-core.exe"),
                System.IO.Path.Combine(baseDir, "..", "netpilot-core.exe"),
                System.IO.Path.Combine(baseDir, "..", "..", "netpilot-core.exe"),
            };
            foreach (var path in candidates)
            {
                var full = System.IO.Path.GetFullPath(path);
                if (!System.IO.File.Exists(full)) continue;
                var psi = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = full,
                    WorkingDirectory = System.IO.Path.GetDirectoryName(full) ?? baseDir,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                };
                System.Diagnostics.Process.Start(psi);
                return;
            }
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine("TryLaunchCore: " + ex.Message);
        }
    }

    private async void NavList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_ready) return;
        if (NavList.SelectedItem is ListBoxItem item && item.Tag is string tag)
            await ShowPageAsync(tag);
    }

    private void ShowPage(string tag) => _ = ShowPageAsync(tag);

    private async Task ShowPageAsync(string tag)
    {
        if (BodyText is null || StatusText is null) return;

        if (!_ipcReady || !_ipc.IsConnected)
        {
            // One reconnect attempt when user navigates while offline.
            if (!_ipcReady)
            {
                await TryConnectCoreAsync();
            }
            if (!_ipcReady)
            {
                BodyText.Text = OfflineText(tag);
                return;
            }
        }

        try
        {
            BodyText.Text = tag switch
            {
                "home" => await BuildHomeAsync(),
                "proxies" => await BuildProxiesAsync(),
                "rules" => await BuildRulesAsync(),
                "connections" => await BuildConnectionsAsync(),
                "logs" => await BuildLogsAsync(),
                "diagnostics" => await BuildDiagnosticsAsync(),
                "subscriptions" => await BuildSubscriptionsAsync(),
                "settings" => await BuildSettingsAsync(),
                _ => $"Page: {tag}"
            };
        }
        catch (Exception ex)
        {
            BodyText.Text = $"Error loading {tag}:\n{ex.Message}";
        }
    }

    private static string OfflineText(string tag) =>
        $"{tag}\n\nCore offline. Start netpilot-core.exe or Start-NetPilot.cmd.";

    private async Task<string> BuildHomeAsync()
    {
        var sb = new StringBuilder();
        sb.AppendLine("NetPilot Home");
        sb.AppendLine();
        sb.AppendLine($"Core IPC: connected");
        sb.AppendLine($"Runtime: {_coreState}");
        try
        {
            // Start TUN (Wintun when DLL present): session → route → pump → connections.
            var startPayload = JsonSerializer.SerializeToElement(new
            {
                name = "NetPilot",
                auto_route = true,
                auto_pump = true,
                require_native = false
            });
            var start = await _ipc.RequestAsync("tunnel.start", startPayload);
            AppendIpcResult(sb, "tunnel.start", start);

            var tun = await _ipc.RequestAsync("tunnel.status");
            AppendIpcResult(sb, "tunnel.status", tun);
            if (tun.TryGetProperty("payload", out var p))
            {
                var running = p.TryGetProperty("running", out var r) && r.ValueKind == JsonValueKind.True;
                var native = p.TryGetProperty("native", out var n) && n.ValueKind == JsonValueKind.True;
                sb.AppendLine();
                sb.AppendLine($"TUN: {(running ? "running" : "stopped")}");
                sb.AppendLine($"Native Wintun: {native}");
                if (p.TryGetProperty("configured_ip", out var ip) && ip.ValueKind != JsonValueKind.Null)
                    sb.AppendLine($"TUN IP: {ip.GetString()}");
                if (p.TryGetProperty("last_error", out var le) && le.ValueKind == JsonValueKind.String)
                    sb.AppendLine($"last_error: {le.GetString()}");
                if (!native)
                    sb.AppendLine("Hint: place wintun.dll next to netpilot-core.exe for Native Wintun: true");
            }
        }
        catch (Exception ex)
        {
            sb.AppendLine($"tunnel: {ex.Message}");
        }
        sb.AppendLine();
        sb.AppendLine("Pipe: \\\\.\\pipe\\netpilot-core");
        return sb.ToString();
    }

    private async Task<string> BuildProxiesAsync()
    {
        var sb = new StringBuilder();
        sb.AppendLine("Proxies");
        sb.AppendLine();
        try
        {
            // NP-INTEGRATION-002: UI → proxy.upsert → Core profile store → proxy.list
            var upsertPayload = JsonSerializer.SerializeToElement(new
            {
                id = "demo-socks5",
                name = "Demo SOCKS5",
                server = "127.0.0.1",
                port = 1080,
                protocol = "socks5"
            });
            var upsert = await _ipc.RequestAsync("proxy.upsert", upsertPayload);
            AppendIpcResult(sb, "proxy.upsert", upsert);

            var list = await _ipc.RequestAsync("proxy.list");
            AppendIpcResult(sb, "proxy.list", list);

            if (list.TryGetProperty("payload", out var p)
                && p.TryGetProperty("items", out var items)
                && items.ValueKind == JsonValueKind.Array)
            {
                sb.AppendLine();
                sb.AppendLine($"nodes: {items.GetArrayLength()}");
                foreach (var it in items.EnumerateArray())
                {
                    var id = it.TryGetProperty("id", out var idv) ? idv.GetString() : "?";
                    var name = it.TryGetProperty("name", out var nv) ? nv.GetString() : id;
                    var server = it.TryGetProperty("server", out var sv) ? sv.GetString() : "";
                    var port = it.TryGetProperty("port", out var pv) ? pv.ToString() : "";
                    var proto = it.TryGetProperty("protocol", out var pr) ? pr.GetString() : "";
                    sb.AppendLine($"• {name} [{id}] {proto} {server}:{port}");
                }
                if (p.TryGetProperty("selected", out var sel) && sel.ValueKind != JsonValueKind.Null)
                    sb.AppendLine($"Selected: {sel.GetString()}");
            }
        }
        catch (Exception ex)
        {
            sb.AppendLine($"IPC error: {ex.Message}");
        }
        return sb.ToString();
    }

    private async Task<string> BuildRulesAsync()
    {
        var sb = new StringBuilder();
        sb.AppendLine("Rules");
        sb.AppendLine();
        try
        {
            // Ensure engine has a minimal rule set so decide is meaningful (real Core IPC).
            var rulesText = "DOMAIN-SUFFIX,google.com,PROXY" + "\n" + "MATCH,DIRECT" + "\n";
            var loadPayload = JsonSerializer.SerializeToElement(new { text = rulesText });
            var load = await _ipc.RequestAsync("rules.load", loadPayload);
            AppendIpcResult(sb, "rules.load", load);

            // Canonical rules.decide payload: domain and/or ip required; port optional.
            var decidePayload = JsonSerializer.SerializeToElement(new
            {
                domain = "www.google.com",
                port = 443
            });
            var decide = await _ipc.RequestAsync("rules.decide", decidePayload);
            AppendIpcResult(sb, "rules.decide", decide);
        }
        catch (Exception ex)
        {
            sb.AppendLine($"IPC error: {ex.Message}");
        }
        return sb.ToString();
    }

    /// Display real Core envelope (ok payload or structured error) — never hide errors.
    private static void AppendIpcResult(StringBuilder sb, string op, JsonElement resp)
    {
        sb.AppendLine($"--- {op} ---");
        if (resp.TryGetProperty("status", out var status))
            sb.AppendLine($"status: {status}");
        if (resp.TryGetProperty("error", out var err) && err.ValueKind != JsonValueKind.Null)
        {
            var kind = err.TryGetProperty("kind", out var k) ? k.GetString() : "?";
            var msg = err.TryGetProperty("message", out var m) ? m.GetString() : err.ToString();
            sb.AppendLine($"error.kind: {kind}");
            sb.AppendLine($"error.message: {msg}");
            return;
        }
        if (resp.TryGetProperty("payload", out var payload))
            sb.AppendLine($"payload: {payload}");
        else
            sb.AppendLine(resp.ToString());
    }

    private async Task<string> BuildLogsAsync()
    {
        var resp = await _ipc.RequestAsync("logs.list");
        var sb = new StringBuilder();
        sb.AppendLine("Logs");
        sb.AppendLine();
        if (resp.TryGetProperty("payload", out var p)
            && p.TryGetProperty("items", out var items)
            && items.ValueKind == JsonValueKind.Array)
        {
            if (items.GetArrayLength() == 0)
                sb.AppendLine("(empty)");
            foreach (var it in items.EnumerateArray())
            {
                var level = it.TryGetProperty("level", out var lv) ? lv.GetString() : "?";
                var msg = it.TryGetProperty("message", out var m) ? m.GetString() : "";
                sb.AppendLine($"[{level}] {msg}");
            }
        }
        return sb.ToString();
    }

    private async Task<string> BuildConnectionsAsync()
    {
        var resp = await _ipc.RequestAsync("connections.list");
        var sb = new StringBuilder();
        sb.AppendLine("Connections");
        sb.AppendLine();
        if (resp.TryGetProperty("payload", out var p)
            && p.TryGetProperty("items", out var items)
            && items.ValueKind == JsonValueKind.Array)
        {
            if (items.GetArrayLength() == 0)
                sb.AppendLine("(no live connections)");
            foreach (var it in items.EnumerateArray())
            {
                var id = it.TryGetProperty("id", out var idv) ? idv.ToString() : "?";
                var dest = it.TryGetProperty("destination", out var d) ? d.GetString() : "";
                var ob = it.TryGetProperty("outbound", out var o) ? o.GetString() : "";
                sb.AppendLine($"#{id} → {dest} via {ob}");
            }
        }
        return sb.ToString();
    }

    private async Task<string> BuildSubscriptionsAsync()
    {
        var sb = new StringBuilder();
        sb.AppendLine("Subscriptions");
        sb.AppendLine();
        try
        {
            // NP-INTEGRATION-003:
            // subscription.add → update(body) → decode/parse → ProxyProfile → engine → proxy.list
            var addPayload = JsonSerializer.SerializeToElement(new
            {
                id = "demo-sub",
                name = "Demo Subscription",
                url = "https://example.com/netpilot-demo-sub"
            });
            var add = await _ipc.RequestAsync("subscription.add", addPayload);
            AppendIpcResult(sb, "subscription.add", add);

            // Sample URI list body (offline path; same pipeline as HTTP fetch).
            var bodyText =
                "trojan://pass@node1.example.com:443?security=tls#Sub-HK-1\n" +
                "ss://YWVzLTI1Ni1nY206cGFzcw@node2.example.com:8388#Sub-SS-1\n";
            var updatePayload = JsonSerializer.SerializeToElement(new
            {
                id = "demo-sub",
                body = bodyText
            });
            var update = await _ipc.RequestAsync("subscription.update", updatePayload);
            AppendIpcResult(sb, "subscription.update", update);

            var list = await _ipc.RequestAsync("subscription.list");
            AppendIpcResult(sb, "subscription.list", list);

            var proxies = await _ipc.RequestAsync("proxy.list");
            AppendIpcResult(sb, "proxy.list (after sub)", proxies);
            if (proxies.TryGetProperty("payload", out var pp)
                && pp.TryGetProperty("items", out var items)
                && items.ValueKind == JsonValueKind.Array)
            {
                sb.AppendLine();
                sb.AppendLine($"proxy nodes visible: {items.GetArrayLength()}");
                foreach (var it in items.EnumerateArray())
                {
                    var id = it.TryGetProperty("id", out var idv) ? idv.GetString() : "?";
                    var name = it.TryGetProperty("name", out var nv) ? nv.GetString() : id;
                    var server = it.TryGetProperty("server", out var sv) ? sv.GetString() : "";
                    var port = it.TryGetProperty("port", out var pv) ? pv.ToString() : "";
                    var proto = it.TryGetProperty("protocol", out var pr) ? pr.GetString() : "";
                    sb.AppendLine($"• {name} [{id}] {proto} {server}:{port}");
                }
            }
        }
        catch (Exception ex)
        {
            sb.AppendLine($"IPC error: {ex.Message}");
        }
        return sb.ToString();
    }

    private async Task<string> BuildDiagnosticsAsync()
    {
        var sb = new StringBuilder();
        sb.AppendLine("Diagnostics");
        sb.AppendLine();
        try
        {
            var ns = await _ipc.RequestAsync("netstack.stats");
            if (ns.TryGetProperty("payload", out var p))
                sb.AppendLine($"Netstack: {p}");
        }
        catch (Exception ex) { sb.AppendLine($"netstack: {ex.Message}"); }
        try
        {
            var probe = await _ipc.RequestAsync("tun.wintun_probe");
            if (probe.TryGetProperty("payload", out var p))
                sb.AppendLine($"Wintun: {p}");
        }
        catch (Exception ex) { sb.AppendLine($"wintun: {ex.Message}"); }

        // Real connectivity: TCP → TLS → HTTPS GET (never treat IPC ping as connectivity).
        sb.AppendLine();
        sb.AppendLine("=== HTTPS connectivity (DIRECT) ===");
        try
        {
            var connPayload = JsonSerializer.SerializeToElement(new
            {
                id = "DIRECT",
                host = "example.com",
                port = 443,
                tls = true
            });
            var conn = await _ipc.RequestAsync("proxy.connectivity", connPayload);
            AppendIpcResult(sb, "proxy.connectivity DIRECT→example.com:443", conn);
            if (conn.TryGetProperty("payload", out var cp))
            {
                var ok = cp.TryGetProperty("ok", out var ov) && ov.ValueKind == JsonValueKind.True;
                sb.AppendLine(ok
                    ? "RESULT: HTTPS path OK (TCP+TLS+HTTP)"
                    : "RESULT: HTTPS path FAILED (see stages above)");
            }
        }
        catch (Exception ex)
        {
            sb.AppendLine($"connectivity: {ex.Message}");
        }

        // If any proxy node exists, probe via first non-demo failure is expected without live proxy.
        try
        {
            var list = await _ipc.RequestAsync("proxy.list");
            if (list.TryGetProperty("payload", out var lp)
                && lp.TryGetProperty("items", out var items)
                && items.ValueKind == JsonValueKind.Array
                && items.GetArrayLength() > 0)
            {
                var first = items[0];
                var id = first.TryGetProperty("id", out var idv) ? idv.GetString() : null;
                if (!string.IsNullOrEmpty(id))
                {
                    sb.AppendLine();
                    sb.AppendLine($"=== HTTPS connectivity via proxy [{id}] ===");
                    var viaPayload = JsonSerializer.SerializeToElement(new
                    {
                        id = id,
                        host = "example.com",
                        port = 443,
                        tls = true
                    });
                    var via = await _ipc.RequestAsync("proxy.connectivity", viaPayload);
                    AppendIpcResult(sb, $"proxy.connectivity {id}→example.com:443", via);
                }
            }
        }
        catch (Exception ex)
        {
            sb.AppendLine($"proxy connectivity: {ex.Message}");
        }

        sb.AppendLine();
        sb.AppendLine("Note: Settings 'ping: ok' only proves IPC. Use stages above for real path.");
        return sb.ToString();
    }

    private async Task<string> BuildSettingsAsync()
    {
        var sb = new StringBuilder();
        sb.AppendLine("Settings");
        sb.AppendLine();
        sb.AppendLine("IPC: \\\\.\\pipe\\netpilot-core");
        try
        {
            var ping = await _ipc.RequestAsync("ping");
            sb.AppendLine($"ping (IPC only): {(ping.TryGetProperty("status", out var s) ? s.GetString() : "?")}");
            sb.AppendLine("For real proxy path use Diagnostics → proxy.connectivity");
        }
        catch (Exception ex) { sb.AppendLine(ex.Message); }
                try
        {
            var sp = await _ipc.RequestAsync("system_proxy.query");
            if (sp.TryGetProperty("payload", out var p))
                sb.AppendLine($"system_proxy: {p}");
        }
        catch (Exception ex) { sb.AppendLine($"system_proxy: {ex.Message}"); }
return sb.ToString();
    }

    private async void RefreshCore_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            _ipc.Disconnect();
        }
        catch { /* ignore */ }
        _ipcReady = false;
        await TryConnectCoreAsync();
        if (NavList.SelectedItem is ListBoxItem { Tag: string tag })
            await ShowPageAsync(tag);
    }

    private void Exit_Click(object sender, RoutedEventArgs e)
    {
        Close();
    }

    private void About_Click(object sender, RoutedEventArgs e)
    {
        MessageBox.Show(
            "NetPilot Desktop (WPF)\nIPC pipe: netpilot-core\nStart Core first, then use Refresh Core.",
            "About NetPilot",
            MessageBoxButton.OK,
            MessageBoxImage.Information);
    }
}
