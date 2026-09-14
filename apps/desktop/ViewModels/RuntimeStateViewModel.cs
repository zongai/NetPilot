// Runtime state binding (NP-051).

using System;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Threading.Tasks;
using NetPilot.Desktop.Models;
using NetPilot.Desktop.Services;

namespace NetPilot.Desktop.ViewModels;

public sealed class RuntimeStateViewModel : INotifyPropertyChanged
{
    private readonly IIpcService _ipc;
    private string _state = "unknown";
    private bool _ready;
    private bool _busy;
    private UserError _error = UserError.None;

    public RuntimeStateViewModel(IIpcService ipc)
    {
        _ipc = ipc;
        _ipc.StateChanged += (_, _) => OnPropertyChanged(nameof(IpcState));
    }

    public string State
    {
        get => _state;
        private set { if (_state != value) { _state = value; OnPropertyChanged(); } }
    }

    public bool Ready
    {
        get => _ready;
        private set { if (_ready != value) { _ready = value; OnPropertyChanged(); } }
    }

    public bool Busy
    {
        get => _busy;
        private set { if (_busy != value) { _busy = value; OnPropertyChanged(); } }
    }

    public UserError Error
    {
        get => _error;
        private set { _error = value; OnPropertyChanged(); OnPropertyChanged(nameof(HasError)); }
    }

    public bool HasError => !Error.IsEmpty;

    public string IpcState => _ipc.State.ToString();

    public async Task RefreshAsync()
    {
        Busy = true;
        Error = UserError.None;
        try
        {
            if (_ipc.State != IpcConnectionState.Connected)
                await _ipc.ConnectAsync();

            var resp = await _ipc.RequestAsync("health.check");
            if (string.Equals(resp.Status, "error", StringComparison.OrdinalIgnoreCase))
            {
                Error = UserError.FromIpc(resp.ErrorKind, resp.ErrorMessage, "health.check");
                return;
            }

            // Lightweight parse without JSON library dependency in skeleton.
            var json = resp.PayloadJson ?? "";
            Ready = json.Contains("\"ready\":true", StringComparison.OrdinalIgnoreCase);
            if (json.Contains("running", StringComparison.OrdinalIgnoreCase))
                State = "running";
            else if (json.Contains("starting", StringComparison.OrdinalIgnoreCase))
                State = "starting";
            else
                State = "unknown";
        }
        catch (Exception ex)
        {
            Error = new UserError
            {
                Kind = UserErrorKind.Internal,
                Message = ex.Message,
                Operation = "health.check",
            };
        }
        finally
        {
            Busy = false;
        }
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
