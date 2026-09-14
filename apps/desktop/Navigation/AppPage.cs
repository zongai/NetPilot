// Navigation page model (NP-050).

namespace NetPilot.Desktop.Navigation;

/// <summary>Logical pages in the desktop shell.</summary>
public enum AppPage
{
    Home,
    Proxies,
    Rules,
    Logs,
    Settings,
}

public static class AppPageMap
{
    public static string Tag(AppPage page) => page switch
    {
        AppPage.Home => "home",
        AppPage.Proxies => "proxies",
        AppPage.Rules => "rules",
        AppPage.Logs => "logs",
        AppPage.Settings => "settings",
        _ => "home",
    };

    public static AppPage FromTag(string? tag) => tag?.ToLowerInvariant() switch
    {
        "proxies" => AppPage.Proxies,
        "rules" => AppPage.Rules,
        "logs" => AppPage.Logs,
        "settings" => AppPage.Settings,
        _ => AppPage.Home,
    };
}
