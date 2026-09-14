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

    public MainWindow()
    {
        InitializeComponent();
        _ready = true;

        if (NavList.Items.Count > 0)
            NavList.SelectedIndex = 0;
        else
            ShowPage("home");

        Loaded += async (_, _) => await TryConnectCoreAsync();
        Closed += (_, _) => _ipc.Dispose();
    }

    private async Task TryConnectCoreAsync()
    {
        try
        {
            StatusText.Text = "Connecting to Core…";
            await _ipc.ConnectAsync(3000);
            var resp = await _ipc.RequestAsync("health.check");
            _coreState = resp.TryGetProperty("payload", out var payload)
                && payload.TryGetProperty("runtime_state", out var st)
                ? st.GetString() ?? "unknown"
                : "connected";
            StatusText.Text = $"Core: {_coreState}";
            if (NavList.SelectedItem is ListBoxItem { Tag: string tag })
                await ShowPageAsync(tag);
        }
        catch (Exception ex)
        {
            _coreState = "disconnected";
            StatusText.Text = "Core offline (start netpilot-core.exe)";
            System.Diagnostics.Debug.WriteLine(ex);
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

        if (!_ipc.IsConnected)
        {
            BodyText.Text = OfflineText(tag);
            return;
        }

        try
        {
            BodyText.Text = tag switch
            {
                "home" => await BuildHomeAsync(),
                "proxies" => await BuildProxiesAsync(),
                "rules" => "Rules\n\nLoad via Core IPC rules.load.\nUse Diagnostics to test rules.decide.",
                "connections" => await BuildConnectionsAsync(),
                "logs" => "Logs\n\nCore structured logs with secret redaction.",
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
            var tun = await _ipc.RequestAsync("tunnel.status");
            if (tun.TryGetProperty("payload", out var p))
            {
                sb.AppendLine($"Tunnel: {(p.TryGetProperty("running", out var r) && r.GetBoolean() ? "running" : "stopped")}");
                if (p.TryGetProperty("native", out var n))
                    sb.AppendLine($"Native Wintun: {n.GetBoolean()}");
                if (p.TryGetProperty("configured_ip", out var ip) && ip.ValueKind != JsonValueKind.Null)
                    sb.AppendLine($"TUN IP: {ip.GetString()}");
            }
        }
        catch { /* ignore */ }
        sb.AppendLine();
        sb.AppendLine("Pipe: \\\\.\\pipe\\netpilot-core");
        return sb.ToString();
    }

    private async Task<string> BuildProxiesAsync()
    {
        var resp = await _ipc.RequestAsync("proxy.list");
        var sb = new StringBuilder();
        sb.AppendLine("Proxies");
        sb.AppendLine();
        if (resp.TryGetProperty("payload", out var p)
            && p.TryGetProperty("items", out var items)
            && items.ValueKind == JsonValueKind.Array)
        {
            if (items.GetArrayLength() == 0)
                sb.AppendLine("(empty — subscription.update or proxy.upsert)");
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
                sb.AppendLine($"\nSelected: {sel.GetString()}");
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
        var resp = await _ipc.RequestAsync("subscription.list");
        var sb = new StringBuilder();
        sb.AppendLine("Subscriptions");
        sb.AppendLine();
        if (resp.TryGetProperty("payload", out var p))
        {
            if (p.TryGetProperty("http_mode", out var hm))
                sb.AppendLine($"HTTP mode: {hm.GetString()}");
            if (p.TryGetProperty("items", out var items) && items.ValueKind == JsonValueKind.Array)
            {
                if (items.GetArrayLength() == 0)
                    sb.AppendLine("(none — subscription.add)");
                foreach (var it in items.EnumerateArray())
                {
                    var id = it.TryGetProperty("id", out var idv) ? idv.GetString() : "?";
                    var name = it.TryGetProperty("name", out var n) ? n.GetString() : id;
                    var url = it.TryGetProperty("url", out var u) ? u.GetString() : "";
                    sb.AppendLine($"• {name} [{id}]");
                    sb.AppendLine($"  {url}");
                }
            }
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
            sb.AppendLine($"ping: {(ping.TryGetProperty("status", out var s) ? s.GetString() : "?")}");
        }
        catch (Exception ex) { sb.AppendLine(ex.Message); }
        return sb.ToString();
    }

    private async void RefreshCore_Click(object sender, RoutedEventArgs e)
    {
        try
        {
            _ipc.Dispose();
        }
        catch { /* ignore */ }
        await TryConnectCoreAsync();
    }
}
