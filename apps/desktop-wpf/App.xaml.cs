using System.Windows;
using System.Windows.Threading;

namespace NetPilot.Desktop.Wpf;

public partial class App : Application
{
    public App()
    {
        DispatcherUnhandledException += OnDispatcherUnhandledException;
        AppDomain.CurrentDomain.UnhandledException += OnUnhandledException;
    }

    private static void OnDispatcherUnhandledException(object sender, DispatcherUnhandledExceptionEventArgs e)
    {
        MessageBox.Show(
            e.Exception.ToString(),
            "NetPilot — unhandled UI exception",
            MessageBoxButton.OK,
            MessageBoxImage.Error);
        e.Handled = true;
    }

    private static void OnUnhandledException(object sender, UnhandledExceptionEventArgs e)
    {
        MessageBox.Show(
            e.ExceptionObject?.ToString() ?? "Unknown error",
            "NetPilot — fatal error",
            MessageBoxButton.OK,
            MessageBoxImage.Error);
    }
}
