// Connection inspector list + detail (NP-054 / NP-055).

using System;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace NetPilot.Desktop.ViewModels;

public sealed class ConnectionItem
{
    public string Id { get; init; } = "";
    public string ProcessName { get; init; } = "";
    public string Destination { get; init; } = "";
    public string Network { get; init; } = "tcp";
    public int Port { get; init; }
    public string Outbound { get; init; } = "DIRECT";
    public string RuleSummary { get; init; } = "";
    public DateTimeOffset StartedAt { get; init; } = DateTimeOffset.UtcNow;
    public long BytesUp { get; set; }
    public long BytesDown { get; set; }
}

public sealed class ConnectionListViewModel : INotifyPropertyChanged
{
    public ObservableCollection<ConnectionItem> Connections { get; } = new();

    private ConnectionItem? _selected;
    public ConnectionItem? Selected
    {
        get => _selected;
        set
        {
            _selected = value;
            OnPropertyChanged();
            OnPropertyChanged(nameof(HasSelection));
            OnPropertyChanged(nameof(DetailText));
        }
    }

    public bool HasSelection => Selected is not null;

    public string DetailText => Selected is null
        ? "Select a connection"
        : $"Id: {Selected.Id}\nProcess: {Selected.ProcessName}\nDest: {Selected.Destination}:{Selected.Port}\nNet: {Selected.Network}\nOutbound: {Selected.Outbound}\nRule: {Selected.RuleSummary}\nUp/Down: {Selected.BytesUp}/{Selected.BytesDown}";

    public void LoadSample()
    {
        Connections.Clear();
        Connections.Add(new ConnectionItem
        {
            Id = "c1",
            ProcessName = "chrome.exe",
            Destination = "www.example.com",
            Port = 443,
            Network = "tcp",
            Outbound = "PROXY",
            RuleSummary = "DOMAIN-SUFFIX,example.com,PROXY",
            BytesUp = 1024,
            BytesDown = 4096,
        });
        Connections.Add(new ConnectionItem
        {
            Id = "c2",
            ProcessName = "system",
            Destination = "10.0.0.1",
            Port = 53,
            Network = "udp",
            Outbound = "DIRECT",
            RuleSummary = "IP-CIDR,10.0.0.0/8,DIRECT",
        });
        Selected = Connections[0];
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    private void OnPropertyChanged([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
