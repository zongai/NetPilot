// Navigation page model (NP-050 / NP-060).

namespace NetPilot.Desktop.Navigation;

public enum AppPage
{
    Home,
    Proxies,
    Rules,
    Connections,
    Logs,
    Diagnostics,
    Settings,
}

public static class AppPageMap
{
    public static string Tag(AppPage page) => page switch
    {
        AppPage.Home => "home",
        AppPage.Proxies => "proxies",
        AppPage.Rules => "rules",
        AppPage.Connections => "connections",
        AppPage.Logs => "logs",
        AppPage.Diagnostics => "diagnostics",
        AppPage.Settings => "settings",
        _ => "home",
    };

    public static AppPage FromTag(string? tag) => tag?.ToLowerInvariant() switch
    {
        "proxies" => AppPage.Proxies,
        "rules" => AppPage.Rules,
        "connections" => AppPage.Connections,
        "logs" => AppPage.Logs,
        "diagnostics" => AppPage.Diagnostics,
        "settings" => AppPage.Settings,
        _ => AppPage.Home,
    };

    /// <summary>Pages required for desktop smoke baseline (NP-060).</summary>
    public static readonly string[] SmokeTags =
    {
        "home", "proxies", "rules", "connections", "logs", "diagnostics", "settings",
    };
}
