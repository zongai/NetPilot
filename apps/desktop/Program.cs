using System;
using System.Runtime.InteropServices;
using System.Threading;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using WinRT;

namespace NetPilot.Desktop;

public static class Program
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int MessageBoxW(IntPtr hWnd, string text, string caption, uint type);

    [STAThread]
    public static void Main(string[] args)
    {
        _ = args;
        try
        {
            ComWrappersSupport.InitializeComWrappers();
            Application.Start(_ =>
            {
                try
                {
                    var context = new DispatcherQueueSynchronizationContext(
                        DispatcherQueue.GetForCurrentThread());
                    SynchronizationContext.SetSynchronizationContext(context);
                    _ = new App();
                }
                catch (Exception ex)
                {
                    MessageBoxW(IntPtr.Zero, ex.ToString(), "NetPilot WinUI — startup failed", 0x10);
                    throw;
                }
            });
        }
        catch (Exception ex)
        {
            MessageBoxW(IntPtr.Zero, ex.ToString(), "NetPilot WinUI — fatal", 0x10);
        }
    }
}
