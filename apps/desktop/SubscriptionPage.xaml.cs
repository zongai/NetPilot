using System;
using System.Text;
using System.Threading.Tasks;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NetPilot.Desktop.Services;
using NetPilot.Desktop.ViewModels;

namespace NetPilot.Desktop;

public sealed partial class SubscriptionPage : Page
{
    public SubscriptionViewModel ViewModel { get; } = new();
    private readonly NamedPipeIpcService _ipc = new();

    public SubscriptionPage()
    {
        InitializeComponent();
        ViewModel.LoadSample();
        SubscriptionList.ItemsSource = ViewModel.Items;
        _ = RunIntegrationAsync();
    }

    private async void AddButton_Click(object sender, RoutedEventArgs e)
    {
        await RunIntegrationAsync().ConfigureAwait(true);
    }

    private async void RefreshAllButton_Click(object sender, RoutedEventArgs e)
    {
        await RunIntegrationAsync().ConfigureAwait(true);
    }

    /// <summary>
    /// NP-INTEGRATION-003: URL/body → decode → parse → ProxyProfile → proxy.list.
    /// </summary>
    private async Task RunIntegrationAsync()
    {
        var sb = new StringBuilder();
        try
        {
            if (_ipc.State != IpcConnectionState.Connected)
                await _ipc.ConnectAsync().ConfigureAwait(true);

            var add = await _ipc
                .RequestAsync(
                    "subscription.add",
                    "{\"id\":\"demo-sub\",\"name\":\"Demo Subscription\",\"url\":\"https://example.com/netpilot-demo-sub\"}")
                .ConfigureAwait(true);
            sb.AppendLine($"add: {add.Status} {add.PayloadJson ?? add.ErrorMessage}");

            var body =
                "trojan://pass@node1.example.com:443?security=tls#Sub-HK-1\\n" +
                "ss://YWVzLTI1Ni1nY206cGFzcw@node2.example.com:8388#Sub-SS-1\\n";
            // Build JSON with real newlines in body value
            body =
                "trojan://pass@node1.example.com:443?security=tls#Sub-HK-1\n" +
                "ss://YWVzLTI1Ni1nY206cGFzcw@node2.example.com:8388#Sub-SS-1\n";
            var bodyJson = System.Text.Json.JsonSerializer.Serialize(body);
            var update = await _ipc
                .RequestAsync("subscription.update", $"{{\"id\":\"demo-sub\",\"body\":{bodyJson}}}")
                .ConfigureAwait(true);
            sb.AppendLine($"update: {update.Status} nodes={update.PayloadJson}");

            var proxies = await _ipc.RequestAsync("proxy.list", null).ConfigureAwait(true);
            sb.AppendLine($"proxy.list: {proxies.PayloadJson}");

            StatusBar.IsOpen = true;
            StatusBar.Title = "subscription → proxies";
            StatusBar.Message = sb.ToString();
            StatusBar.Severity =
                update.Status == "ok" ? InfoBarSeverity.Success : InfoBarSeverity.Error;
        }
        catch (Exception ex)
        {
            StatusBar.IsOpen = true;
            StatusBar.Title = "IPC error";
            StatusBar.Message = ex.Message;
            StatusBar.Severity = InfoBarSeverity.Error;
        }
    }
}
