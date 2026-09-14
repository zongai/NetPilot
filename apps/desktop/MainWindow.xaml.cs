// NetPilot Desktop main window + navigation (NP-049 / NP-050).
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace NetPilot.Desktop;

public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        // Default landing page
        NavigateTo("home");
    }

    private void NavView_SelectionChanged(
        NavigationView sender,
        NavigationViewSelectionChangedEventArgs args)
    {
        if (args.IsSettingsSelected)
        {
            NavigateTo("settings");
            return;
        }

        if (args.SelectedItem is NavigationViewItem item && item.Tag is string tag)
        {
            NavigateTo(tag);
        }
    }

    /// <summary>
    /// Page model router (NP-050). Pages are placeholders until full XAML pages land.
    /// </summary>
    public void NavigateTo(string pageTag)
    {
        var title = pageTag switch
        {
            "home" => "Home",
            "proxies" => "Proxies",
            "rules" => "Rules",
            "logs" => "Logs",
            "settings" => "Settings",
            _ => "Unknown",
        };

        // Skeleton: single content text until Page types are added.
        ContentFrame.Content = new TextBlock
        {
            Text = $"NetPilot — {title}",
            FontSize = 24,
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center,
        };
    }
}
