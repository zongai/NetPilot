// Proxy node/group UI model (NP-052).

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace NetPilot.Desktop.ViewModels;

public sealed class ProxyNodeItem
{
    public string Id { get; init; } = "";
    public string Name { get; init; } = "";
    public string Protocol { get; init; } = "";
    public string Server { get; init; } = "";
    public int Port { get; init; }
    public string LatencyText { get; set; } = "—";
}

public sealed class ProxyGroupItem
{
    public string Id { get; init; } = "";
    public string Name { get; init; } = "";
    public string SelectPolicy { get; init; } = "manual";
    public string? SelectedId { get; set; }
    public ObservableCollection<string> MemberIds { get; } = new();
}

public sealed class ProxyListViewModel : INotifyPropertyChanged
{
    public ObservableCollection<ProxyNodeItem> Nodes { get; } = new();
    public ObservableCollection<ProxyGroupItem> Groups { get; } = new();

    private ProxyNodeItem? _selectedNode;
    public ProxyNodeItem? SelectedNode
    {
        get => _selectedNode;
        set { _selectedNode = value; OnPropertyChanged(); }
    }

    private ProxyGroupItem? _selectedGroup;
    public ProxyGroupItem? SelectedGroup
    {
        get => _selectedGroup;
        set { _selectedGroup = value; OnPropertyChanged(); }
    }

    /// <summary>Design-time / smoke sample data (no secrets).</summary>
    public void LoadSample()
    {
        Nodes.Clear();
        Groups.Clear();
        Nodes.Add(new ProxyNodeItem
        {
            Id = "p1",
            Name = "Sample SOCKS",
            Protocol = "socks5",
            Server = "127.0.0.1",
            Port = 1080,
        });
        var g = new ProxyGroupItem
        {
            Id = "g1",
            Name = "Default",
            SelectPolicy = "manual",
            SelectedId = "p1",
        };
        g.MemberIds.Add("p1");
        Groups.Add(g);
        SelectedNode = Nodes[0];
        SelectedGroup = g;
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    private void OnPropertyChanged([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
