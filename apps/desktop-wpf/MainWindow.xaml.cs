using System.Windows;
using System.Windows.Controls;

namespace NetPilot.Desktop.Wpf;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        ShowPage("home");
    }

    private void NavList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (NavList.SelectedItem is ListBoxItem item && item.Tag is string tag)
            ShowPage(tag);
    }

    private void ShowPage(string tag)
    {
        BodyText.Text = tag switch
        {
            "home" => "NetPilot Home\n\nCore: netpilot-core.exe (rules, DNS, TUN, subscription)\nIPC: named pipe (loopback until connected)\n\nStart Core alongside this GUI for full control plane.",
            "proxies" => "Proxies\n\nManaged by Core ProxyProfile / groups.\nUse subscription update to import airport nodes.",
            "rules" => "Rules\n\nClash-like rule engine in Core (domain/IP/process).",
            "connections" => "Connections\n\nLive connection inspector (Core diagnostics).",
            "logs" => "Logs\n\nStructured logs with secret redaction.",
            "diagnostics" => "Diagnostics\n\nDNS/TCP probes and reports from Core.",
            "subscriptions" => "Subscriptions (S11)\n\nAirport URL → fetch → decode → parse → ProxyProfile.\nFormats: URI list, Clash YAML, Sing-box JSON.",
            "settings" => "Settings\n\nIPC endpoint, theme, update policy (later).",
            _ => tag,
        };
        StatusText.Text = $"Page: {tag}";
    }

    private void Exit_Click(object sender, RoutedEventArgs e) => Close();
}
