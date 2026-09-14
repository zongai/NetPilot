// Rule list UI model (NP-053).

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace NetPilot.Desktop.ViewModels;

public sealed class RuleItem
{
    public int Index { get; init; }
    public string MatcherKind { get; init; } = "";
    public string Pattern { get; init; } = "";
    public string Outbound { get; init; } = "";
    public int Priority { get; init; } = 1000;
    public string? Source { get; init; }
}

public sealed class RuleListViewModel : INotifyPropertyChanged
{
    public ObservableCollection<RuleItem> Rules { get; } = new();

    private RuleItem? _selected;
    public RuleItem? Selected
    {
        get => _selected;
        set { _selected = value; OnPropertyChanged(); }
    }

    public void LoadSample()
    {
        Rules.Clear();
        Rules.Add(new RuleItem
        {
            Index = 0,
            MatcherKind = "domain_suffix",
            Pattern = "google.com",
            Outbound = "PROXY",
            Priority = 10,
            Source = "DOMAIN-SUFFIX,google.com,PROXY",
        });
        Rules.Add(new RuleItem
        {
            Index = 1,
            MatcherKind = "ip_cidr",
            Pattern = "10.0.0.0/8",
            Outbound = "DIRECT",
            Priority = 20,
            Source = "IP-CIDR,10.0.0.0/8,DIRECT",
        });
        Rules.Add(new RuleItem
        {
            Index = 2,
            MatcherKind = "match",
            Pattern = "*",
            Outbound = "DIRECT",
            Priority = 1000,
            Source = "MATCH,DIRECT",
        });
        Selected = Rules[0];
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    private void OnPropertyChanged([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
