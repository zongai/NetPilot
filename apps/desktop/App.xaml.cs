using Microsoft.UI.Xaml;

namespace NetPilot.Desktop;

public partial class App : Application
{
    private Window? _window;

    public App()
    {
        UnhandledException += (_, e) =>
        {
            System.Diagnostics.Debug.WriteLine(e.Message);
            e.Handled = true;
        };
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();
        _window.Closed += (_, _) => { _window = null; };
        _window.Activate();
    }
}
