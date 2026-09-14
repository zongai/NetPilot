// NetPilot Desktop main window + navigation (NP-049…NP-060).
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NetPilot.Desktop.Navigation;
using NetPilot.Desktop.Services;
using NetPilot.Desktop.ViewModels;

namespace NetPilot.Desktop;

public sealed partial class MainWindow : Window
{
    // Prefer real named pipe; fall back to loopback for design-time without Core.
    private readonly IIpcService _ipc = CreateIpc();

    private static IIpcService CreateIpc()
    {
        try
        {
            var pipe = new NamedPipeIpcService();
            // Connect is async — RuntimeStateViewModel will call ConnectAsync via health.
            return pipe;
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

        var body = page switch
        {
            AppPage.Home =>
                $"Home\nRuntime: {Runtime.State}\nReady: {Runtime.Ready}\nIPC: {Runtime.IpcState}",
            AppPage.Proxies =>
                $"Proxies\nNodes: {Proxies.Nodes.Count}\nGroups: {Proxies.Groups.Count}\nSelected: {Proxies.SelectedNode?.Name}",
            AppPage.Rules =>
                $"Rules\nCount: {Rules.Rules.Count}\nSelected: {Rules.Selected?.Source}",
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
}
