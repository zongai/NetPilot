// NetPilot Desktop main window + navigation (NP-049…NP-060 + INTEGRATION-001 Rules).
using System;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NetPilot.Desktop.Navigation;
using NetPilot.Desktop.Services;
using NetPilot.Desktop.ViewModels;

namespace NetPilot.Desktop;

public sealed partial class MainWindow : Window
{
    private readonly IIpcService _ipc = CreateIpc();

    private static IIpcService CreateIpc()
    {
        try
        {
            return new NamedPipeIpcService();
        }
        catch
        {
            return new LoopbackIpcService();
        }
    }

    public RuntimeStateViewModel Runtime { get; }
    public ProxyListViewModel Proxies { get; } = new();
    public RuleListViewModel Rules { get; } = new();
    public ConnectionListViewModel Connections { get; } = new();
    public LogViewModel Logs { get; } = new();
    public DiagnosticsViewModel Diagnostics { get; }

    public MainWindow()
    {
        InitializeComponent();
        Runtime = new RuntimeStateViewModel(_ipc);
        Diagnostics = new DiagnosticsViewModel(_ipc);

        Proxies.LoadSample();
        Rules.LoadSample();
        Connections.LoadSample();
        Logs.LoadSample();

        NavigateTo(AppPageMap.Tag(AppPage.Home));
        _ = Runtime.RefreshAsync();
    }

    private void NavView_SelectionChanged(
        NavigationView sender,
        NavigationViewSelectionChangedEventArgs args)
    {
        if (args.IsSettingsSelected)
        {
            NavigateTo(AppPageMap.Tag(AppPage.Settings));
            return;
        }

        if (args.SelectedItem is NavigationViewItem item && item.Tag is string tag)
            NavigateTo(tag);
    }

    public void NavigateTo(string pageTag)
    {
        var page = AppPageMap.FromTag(pageTag);
        if (page == AppPage.Subscriptions)
        {
            ContentFrame.Content = new SubscriptionPage();
            return;
        }

        if (page == AppPage.Rules)
        {
            var block = new TextBlock
            {
                Text = "Rules\nLoading rules.decide…",
                FontSize = 18,
                TextWrapping = TextWrapping.Wrap,
                Margin = new Thickness(8),
            };
            ContentFrame.Content = block;
            _ = LoadRulesPageAsync(block);
            return;
        }

        var body = page switch
        {
            AppPage.Home =>
                $"Home\nRuntime: {Runtime.State}\nReady: {Runtime.Ready}\nIPC: {Runtime.IpcState}",
            AppPage.Proxies =>
                $"Proxies\nNodes: {Proxies.Nodes.Count}\nGroups: {Proxies.Groups.Count}\nSelected: {Proxies.SelectedNode?.Name}",
            AppPage.Connections => Connections.DetailText,
            AppPage.Logs => $"Logs\nLines: {Logs.Lines.Count}",
            AppPage.Diagnostics => $"Diagnostics items: {Diagnostics.Items.Count}",
            AppPage.Settings => "Settings\n(IPC endpoint, theme — later)",
            _ => pageTag,
        };

        ContentFrame.Content = new TextBlock
        {
            Text = $"NetPilot — {body}",
            FontSize = 18,
            TextWrapping = TextWrapping.Wrap,
            Margin = new Thickness(8),
        };
    }

    /// <summary>
    /// Real Core IPC: rules.load then rules.decide with canonical payload.
    /// </summary>
    private async Task LoadRulesPageAsync(TextBlock block)
    {
        var sb = new StringBuilder();
        sb.AppendLine("Rules");
        sb.AppendLine();
        try
        {
            if (_ipc.State != IpcConnectionState.Connected)
                await _ipc.ConnectAsync().ConfigureAwait(true);

            var rulesText = "DOMAIN-SUFFIX,google.com,PROXY\nMATCH,DIRECT\n";
            var load = await _ipc
                .RequestAsync("rules.load", $"{{\"text\":{System.Text.Json.JsonSerializer.Serialize(rulesText)}}}")
                .ConfigureAwait(true);
            AppendDto(sb, "rules.load", load);

            // Canonical payload: domain and/or ip required; port optional.
            var decide = await _ipc
                .RequestAsync("rules.decide", "{\"domain\":\"www.google.com\",\"port\":443}")
                .ConfigureAwait(true);
            AppendDto(sb, "rules.decide", decide);
        }
        catch (Exception ex)
        {
            sb.AppendLine($"IPC error: {ex.Message}");
        }

        block.Text = sb.ToString();
    }

    private static void AppendDto(StringBuilder sb, string op, IpcEnvelopeDto resp)
    {
        sb.AppendLine($"--- {op} ---");
        sb.AppendLine($"status: {resp.Status}");
        if (!string.IsNullOrEmpty(resp.ErrorKind) || !string.IsNullOrEmpty(resp.ErrorMessage))
        {
            sb.AppendLine($"error.kind: {resp.ErrorKind}");
            sb.AppendLine($"error.message: {resp.ErrorMessage}");
            return;
        }
        sb.AppendLine($"payload: {resp.PayloadJson}");
    }
}
