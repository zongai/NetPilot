// Unpackaged WinUI entry (DISABLE_XAML_GENERATED_MAIN).
using System;
using System.Threading;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using WinRT;

namespace NetPilot.Desktop;

public static class Program
{
    [STAThread]
    public static void Main(string[] args)
    {
        _ = args;
        ComWrappersSupport.InitializeComWrappers();
        Application.Start(_ =>
        {
            var context = new DispatcherQueueSynchronizationContext(
                DispatcherQueue.GetForCurrentThread());
            SynchronizationContext.SetSynchronizationContext(context);
            _ = new App();
        });
    }
}
