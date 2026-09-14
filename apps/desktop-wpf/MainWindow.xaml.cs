using System.Windows;
using System.Windows.Controls;

namespace NetPilot.Desktop.Wpf;

public partial class MainWindow : Window
{
    private bool _ready;

    public MainWindow()
    {
        InitializeComponent();
        _ready = true;

        if (NavList.Items.Count > 0)
        {
            NavList.SelectedIndex = 0;
        }
        else
        {
            ShowPage("home");
        }
    }

    private void NavList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (!_ready)
        {
            return;
        }

        if (NavList.SelectedItem is ListBoxItem item && item.Tag is string tag)
        {
            ShowPage(tag);
        }
    }

    private void ShowPage(string tag)
    {
        if (BodyText is null || StatusText is null)
        {
            return;
        }

        BodyText.Text = tag switch
        {
            "home" =>
                "NetPilot Home\n\n" +
                "Core: netpilot-core.exe (rules, DNS, TUN, subscription)\n" +
                "IPC: named pipe (loopback until connected)\n\n" +
                "Start Core alongside this GUI for the full control plane.\n" +
                "This window should stay open until you close it.",
            "proxies" =>
                "Proxies\n\nManaged by Core ProxyProfile / groups.\n" +
                "Use subscription update to import airport nodes.",
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
                "Settings\n\nIPC endpoint, theme, update policy (later).",
            _ => tag,
        };
        StatusText.Text = $"Page: {tag}";
    }

    private void Exit_Click(object sender, RoutedEventArgs e) => Close();

    private void About_Click(object sender, RoutedEventArgs e)
    {
        MessageBox.Show(
            "NetPilot Desktop (WPF)\nVersion 0.1.1\n\nUI shell for netpilot-core.exe",
            "About NetPilot",
            MessageBoxButton.OK,
            MessageBoxImage.Information);
    }
}
