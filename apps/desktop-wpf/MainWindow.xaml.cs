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
                ShowPage(tag);
        }
        catch (Exception ex)
        {
            _coreState = "disconnected";
            StatusText.Text = "Core offline (start netpilot-core.exe)";
            System.Diagnostics.Debug.WriteLine(ex);
        }
    }

    private void NavList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_ready) return;
        if (NavList.SelectedItem is ListBoxItem item && item.Tag is string tag)
            ShowPage(tag);
    }

    private void ShowPage(string tag)
    {
        if (BodyText is null || StatusText is null) return;

        BodyText.Text = tag switch
        {
            "home" =>
                "NetPilot Home\n\n" +
                $"Core IPC: {(_ipc.IsConnected ? "connected" : "disconnected")}\n" +
                $"Runtime: {_coreState}\n" +
                "Pipe: \\\\.\\pipe\\netpilot-core\n\n" +
                "Start netpilot-core.exe first (or use Start-NetPilot.cmd).\n" +
                "Use Help → Refresh Core to reconnect.",
            "proxies" =>
                "Proxies\n\nManaged by Core ProxyProfile / groups.\n" +
                "Live list will bind to IPC in a later increment.",
            "rules" =>
                "Rules\n\nClash-like rule engine in Core (domain / IP / process).",
            "connections" =>
                "Connections\n\nLive connection inspector (Core diagnostics).",
            "logs" =>
                "Logs\n\nStructured logs with secret redaction.",
            "diagnostics" =>
                "Diagnostics\n\nDNS / TCP probes and reports from Core.",
            "subscriptions" =>
                "Subscriptions (S11)\n\n" +
                "Airport URL → fetch → decode → parse → ProxyProfile.\n" +
                "Formats: URI list, Clash YAML, Sing-box JSON.",
            "settings" =>
                "Settings\n\nIPC: \\\\.\\pipe\\netpilot-core\n" +
                "Operations: health.check, runtime.state, runtime.shutdown, ping",
            _ => tag,
        };
        StatusText.Text = $"Page: {tag} | Core: {_coreState}";
    }

    private void Exit_Click(object sender, RoutedEventArgs e) => Close();

    private async void About_Click(object sender, RoutedEventArgs e)
    {
        string extra = _ipc.IsConnected ? $"\nCore state: {_coreState}" : "\nCore: not connected";
        MessageBox.Show(
            "NetPilot Desktop (WPF)\nVersion 0.1.2\n" + extra,
            "About NetPilot",
            MessageBoxButton.OK,
            MessageBoxImage.Information);
        await Task.CompletedTask;
    }

    private async void RefreshCore_Click(object sender, RoutedEventArgs e)
    {
        await TryConnectCoreAsync();
    }
}
