// NetPilot Desktop shell (NP-049). Network interception stays in Core.
using Microsoft.UI.Xaml;

namespace NetPilot.Desktop;

public partial class App : Application
{
    private Window? _window;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();
        _window.Activate();
    }
}
