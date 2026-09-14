// Real named-pipe IPC client (Desktop → Core).
using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace NetPilot.Desktop.Services;

public sealed class NamedPipeIpcService : IIpcService, IDisposable
{
    public const string PipeName = "netpilot-core";

    private readonly object _gate = new();
    private IpcConnectionState _state = IpcConnectionState.Disconnected;
    private NamedPipeClientStream? _pipe;
    private StreamReader? _reader;
    private StreamWriter? _writer;

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

    public async Task ConnectAsync(CancellationToken ct = default)
    {
        Disconnect();
        State = IpcConnectionState.Connecting;
        try
        {
            var pipe = new NamedPipeClientStream(".", PipeName, PipeDirection.InOut, PipeOptions.Asynchronous);
            await pipe.ConnectAsync(5000, ct).ConfigureAwait(false);
            _pipe = pipe;
            _reader = new StreamReader(pipe, Encoding.UTF8, false, 4096, leaveOpen: true);
            _writer = new StreamWriter(pipe, Encoding.UTF8, 4096, leaveOpen: true) { AutoFlush = true };
            State = IpcConnectionState.Connected;
        }
        catch
        {
            State = IpcConnectionState.Faulted;
            throw;
        }
    }

    public Task DisconnectAsync()
    {
        Disconnect();
        State = IpcConnectionState.Disconnected;
        return Task.CompletedTask;
    }

    private void Disconnect()
    {
        try { _writer?.Dispose(); } catch { }
        try { _reader?.Dispose(); } catch { }
        try { _pipe?.Dispose(); } catch { }
        _writer = null;
        _reader = null;
        _pipe = null;
    }

    public async Task<IpcEnvelopeDto> RequestAsync(string operation, string? payloadJson = null, CancellationToken ct = default)
    {
        if (State != IpcConnectionState.Connected || _writer is null || _reader is null)
        {
            return new IpcEnvelopeDto
            {
                Kind = "response",
                RequestId = Guid.NewGuid().ToString("N"),
                Operation = operation,
                Status = "error",
                ErrorKind = "unavailable",
                ErrorMessage = "IPC not connected",
            };
        }

        var req = new Dictionary<string, object?>
        {
            ["protocol_version"] = 1,
            ["kind"] = "request",
            ["request_id"] = Guid.NewGuid().ToString("N"),
            ["operation"] = operation,
        };
        if (!string.IsNullOrEmpty(payloadJson))
        {
            using var doc = JsonDocument.Parse(payloadJson);
            req["payload"] = doc.RootElement.Clone();
        }

        await _writer.WriteLineAsync(JsonSerializer.Serialize(req).AsMemory(), ct).ConfigureAwait(false);
        var line = await _reader.ReadLineAsync(ct).ConfigureAwait(false);
        if (line is null)
        {
            State = IpcConnectionState.Faulted;
            return new IpcEnvelopeDto
            {
                Kind = "response",
                Operation = operation,
                Status = "error",
                ErrorKind = "disconnected",
                ErrorMessage = "Core closed pipe",
            };
        }

        using var resp = JsonDocument.Parse(line);
        var root = resp.RootElement;
        return new IpcEnvelopeDto
        {
            ProtocolVersion = root.TryGetProperty("protocol_version", out var pv) ? pv.GetUInt32() : 1,
            Kind = root.TryGetProperty("kind", out var k) ? k.GetString() ?? "response" : "response",
            RequestId = root.TryGetProperty("request_id", out var id) ? id.GetString() ?? "" : "",
            Operation = root.TryGetProperty("operation", out var op) ? op.GetString() : operation,
            Status = root.TryGetProperty("status", out var st) ? st.GetString() : null,
            PayloadJson = root.TryGetProperty("payload", out var pl) ? pl.GetRawText() : null,
            ErrorKind = root.TryGetProperty("error", out var err) && err.TryGetProperty("kind", out var ek) ? ek.GetString() : null,
            ErrorMessage = root.TryGetProperty("error", out var err2) && err2.TryGetProperty("message", out var em) ? em.GetString() : null,
        };
    }

    public void Dispose() => Disconnect();
}
