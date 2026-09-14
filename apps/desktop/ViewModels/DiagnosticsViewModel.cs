// Diagnostics UI model (NP-057).

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using NetPilot.Desktop.Services;

namespace NetPilot.Desktop.ViewModels;

public sealed class DiagnosticItem
{
    public string Name { get; init; } = "";
    public string Status { get; init; } = "unknown";
    public string Detail { get; init; } = "";
}

public sealed class DiagnosticsViewModel : INotifyPropertyChanged
{
    private readonly IIpcService _ipc;

    public DiagnosticsViewModel(IIpcService ipc) => _ipc = ipc;

    public ObservableCollection<DiagnosticItem> Items { get; } = new();

    public async Task RunAsync()
    {
        Items.Clear();
        Items.Add(new DiagnosticItem
        {
            Name = "IPC",
            Status = _ipc.State.ToString(),
            Detail = "Named pipe client state",
        });

        if (_ipc.State != IpcConnectionState.Connected)
            await _ipc.ConnectAsync();

        var health = await _ipc.RequestAsync("health.check");
        Items.Add(new DiagnosticItem
        {
            Name = "Core health",
            Status = health.Status ?? "?",
            Detail = health.PayloadJson ?? health.ErrorMessage ?? "",
        });

        Items.Add(new DiagnosticItem
        {
            Name = "Protocol",
            Status = "1",
            Detail = "IPC protocol_version",
        });
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    private void OnPropertyChanged([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
