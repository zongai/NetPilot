using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;

namespace NetPilot.Desktop.Wpf;

/// <summary>Newline-delimited JSON IPC client to netpilot-core named pipe.</summary>
/// <remarks>
/// A single duplex pipe must serialize request/response pairs. Concurrent
/// RequestAsync without a lock causes response cross-wiring (e.g. proxy.list
/// receiving tun.wintun_probe payload).
/// </remarks>
public sealed class CoreIpcClient : IDisposable
{
    public const string PipeName = "netpilot-core";

    private readonly SemaphoreSlim _gate = new(1, 1);
    private NamedPipeClientStream? _pipe;
    private StreamReader? _reader;
    private StreamWriter? _writer;

    public bool IsConnected => _pipe is { IsConnected: true };

    public async Task ConnectAsync(int timeoutMs = 5000, CancellationToken ct = default)
    {
        await _gate.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            DisconnectUnlocked();
            var pipe = new NamedPipeClientStream(
                ".",
                PipeName,
                PipeDirection.InOut,
                PipeOptions.Asynchronous);
            await pipe.ConnectAsync(timeoutMs, ct).ConfigureAwait(false);
            _pipe = pipe;
            _reader = new StreamReader(pipe, Encoding.UTF8, detectEncodingFromByteOrderMarks: false, bufferSize: 4096, leaveOpen: true);
            _writer = new StreamWriter(pipe, Encoding.UTF8, bufferSize: 4096, leaveOpen: true) { AutoFlush = true };
        }
        finally
        {
            _gate.Release();
        }
    }

    public void Disconnect()
    {
        _gate.Wait();
        try
        {
            DisconnectUnlocked();
        }
        finally
        {
            _gate.Release();
        }
    }

    private void DisconnectUnlocked()
    {
        try { _writer?.Dispose(); } catch { /* ignore */ }
        try { _reader?.Dispose(); } catch { /* ignore */ }
        try { _pipe?.Dispose(); } catch { /* ignore */ }
        _writer = null;
        _reader = null;
        _pipe = null;
    }

    public async Task<JsonElement> RequestAsync(string operation, JsonElement? payload = null, CancellationToken ct = default)
    {
        await _gate.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            if (_writer is null || _reader is null || _pipe is null || !_pipe.IsConnected)
                throw new InvalidOperationException("IPC not connected");

            var requestId = Guid.NewGuid().ToString("N");
            var req = new Dictionary<string, object?>
            {
                ["protocol_version"] = 1u,
                ["kind"] = "request",
                ["request_id"] = requestId,
                ["operation"] = operation,
            };
            if (payload is { } p)
                req["payload"] = p;

            var line = JsonSerializer.Serialize(req);
            await _writer.WriteLineAsync(line.AsMemory(), ct).ConfigureAwait(false);

            // Drain until matching request_id (tolerate stray lines; max 8).
            for (var i = 0; i < 8; i++)
            {
                var responseLine = await _reader.ReadLineAsync(ct).ConfigureAwait(false)
                    ?? throw new EndOfStreamException("Core closed the pipe");
                using var doc = JsonDocument.Parse(responseLine);
                var root = doc.RootElement;
                var rid = root.TryGetProperty("request_id", out var ridEl) ? ridEl.GetString() : null;
                var op = root.TryGetProperty("operation", out var opEl) ? opEl.GetString() : null;

                if (rid == requestId)
                {
                    // Optional: warn if operation mismatches but still return (Core echoes op).
                    if (op is not null && !string.Equals(op, operation, StringComparison.Ordinal))
                    {
                        // Keep payload but surface mismatch in a synthetic wrapper is overkill;
                        // matching request_id is the authoritative pairing.
                    }
                    return root.Clone();
                }
                // Mismatched response — skip (stale concurrent reply).
            }

            throw new InvalidOperationException(
                $"IPC response mismatch for operation '{operation}' (request_id={requestId}): no matching reply");
        }
        finally
        {
            _gate.Release();
        }
    }

    public void Dispose()
    {
        Disconnect();
        _gate.Dispose();
    }
}
