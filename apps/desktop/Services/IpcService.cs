// UI IPC service abstraction (NP-059).
// Transport details stay in Core; Desktop only speaks envelopes over the named pipe.

using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;

namespace NetPilot.Desktop.Services;

/// <summary>Connection state of the Desktop → Core pipe.</summary>
public enum IpcConnectionState
{
    Disconnected,
    Connecting,
    Connected,
    Faulted,
}

/// <summary>Minimal envelope mirror of crates/ipc IpcEnvelope (JSON field names).</summary>
public sealed class IpcEnvelopeDto
{
    public uint ProtocolVersion { get; set; } = 1;
    public string Kind { get; set; } = "request";
    public string RequestId { get; set; } = "";
    public string? Operation { get; set; }
    public string? Status { get; set; }
    public string? PayloadJson { get; set; }
    public string? ErrorKind { get; set; }
    public string? ErrorMessage { get; set; }
}

public interface IIpcService
{
    IpcConnectionState State { get; }
    event EventHandler? StateChanged;
    event EventHandler<IpcEnvelopeDto>? EventReceived;

    Task ConnectAsync(CancellationToken ct = default);
    Task DisconnectAsync();
    Task<IpcEnvelopeDto> RequestAsync(string operation, string? payloadJson = null, CancellationToken ct = default);
}

/// <summary>
/// Skeleton implementation: in-memory loopback until OS named-pipe client is wired.
/// Safe for UI design-time and smoke checks without a running Core.
/// </summary>
public sealed class LoopbackIpcService : IIpcService
{
    private readonly object _gate = new();
    private IpcConnectionState _state = IpcConnectionState.Disconnected;

    public IpcConnectionState State
    {
        get { lock (_gate) return _state; }
        private set
        {
            lock (_gate) _state = value;
            StateChanged?.Invoke(this, EventArgs.Empty);
        }
    }

    public event EventHandler? StateChanged;
    public event EventHandler<IpcEnvelopeDto>? EventReceived;

    public Task ConnectAsync(CancellationToken ct = default)
    {
        State = IpcConnectionState.Connecting;
        State = IpcConnectionState.Connected;
        return Task.CompletedTask;
    }

    public Task DisconnectAsync()
    {
        State = IpcConnectionState.Disconnected;
        return Task.CompletedTask;
    }

    public Task<IpcEnvelopeDto> RequestAsync(string operation, string? payloadJson = null, CancellationToken ct = default)
    {
        if (State != IpcConnectionState.Connected)
        {
            return Task.FromResult(new IpcEnvelopeDto
            {
                Kind = "response",
                RequestId = Guid.NewGuid().ToString("N"),
                Operation = operation,
                Status = "error",
                ErrorKind = "unavailable",
                ErrorMessage = "IPC not connected",
            });
        }

        // Deterministic stubs for smoke / design-time.
        var okPayload = operation switch
        {
            "health.check" => "{\"runtime_state\":\"running\",\"ready\":true,\"protocol_version\":1}",
            "health.ready" => "{\"ready\":true}",
            "runtime.state" => "{\"state\":\"running\"}",
            _ => payloadJson ?? "{}",
        };

        return Task.FromResult(new IpcEnvelopeDto
        {
            Kind = "response",
            RequestId = Guid.NewGuid().ToString("N"),
            Operation = operation,
            Status = "ok",
            PayloadJson = okPayload,
        });
    }

    /// <summary>Raise a synthetic Core event (tests / previews).</summary>
    public void PublishEvent(IpcEnvelopeDto evt) => EventReceived?.Invoke(this, evt);
}
