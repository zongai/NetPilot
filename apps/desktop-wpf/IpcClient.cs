using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;

namespace NetPilot.Desktop.Wpf;

/// <summary>Newline-delimited JSON IPC client to netpilot-core named pipe.</summary>
/// <remarks>
/// Uses raw pipe byte I/O (not StreamReader/Writer) to avoid buffering stalls with
/// the Core byte-mode PIPE_NOWAIT server. Serializes request/response under a lock.
/// </remarks>
public sealed class CoreIpcClient : IDisposable
{
    public const string PipeName = "netpilot-core";

    private readonly SemaphoreSlim _gate = new(1, 1);
    private NamedPipeClientStream? _pipe;
    private readonly byte[] _readBuf = new byte[64 * 1024];
    private readonly List<byte> _lineBuf = new();

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
            try
            {
                pipe.ReadMode = PipeTransmissionMode.Byte;
            }
            catch
            {
                // Some hosts ignore ReadMode; byte is default for this pipe.
            }
            _pipe = pipe;
            _lineBuf.Clear();
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
        try { _pipe?.Dispose(); } catch { /* ignore */ }
        _pipe = null;
        _lineBuf.Clear();
    }

    public async Task<JsonElement> RequestAsync(string operation, JsonElement? payload = null, CancellationToken ct = default)
    {
        // Overall per-request budget so UI never hangs forever on "Connecting…"
        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        if (!cts.IsCancellationRequested)
            cts.CancelAfter(TimeSpan.FromSeconds(12));
        var token = cts.Token;

        await _gate.WaitAsync(token).ConfigureAwait(false);
        try
        {
            if (_pipe is null || !_pipe.IsConnected)
                throw new InvalidOperationException("IPC not connected");

            var requestId = Guid.NewGuid().ToString("N");
            var req = new Dictionary<string, object?>
            {
                ["protocol_version"] = 1,
                ["kind"] = "request",
                ["request_id"] = requestId,
                ["operation"] = operation,
            };
            if (payload is { } p)
                req["payload"] = p;

            var json = JsonSerializer.Serialize(req);
            var bytes = Encoding.UTF8.GetBytes(json + "\n");
            await _pipe.WriteAsync(bytes.AsMemory(0, bytes.Length), token).ConfigureAwait(false);
            await _pipe.FlushAsync(token).ConfigureAwait(false);

            for (var i = 0; i < 8; i++)
            {
                var responseLine = await ReadLineAsync(_pipe, token).ConfigureAwait(false);
                using var doc = JsonDocument.Parse(responseLine);
                var root = doc.RootElement;
                var rid = root.TryGetProperty("request_id", out var ridEl) ? ridEl.GetString() : null;
                if (rid == requestId)
                    return root.Clone();
            }

            throw new InvalidOperationException(
                $"IPC response mismatch for '{operation}' (request_id={requestId})");
        }
        finally
        {
            _gate.Release();
        }
    }

    private async Task<string> ReadLineAsync(NamedPipeClientStream pipe, CancellationToken ct)
    {
        while (true)
        {
            ct.ThrowIfCancellationRequested();

            // Prefer data already buffered.
            for (var i = 0; i < _lineBuf.Count; i++)
            {
                if (_lineBuf[i] == (byte)'\n')
                {
                    var slice = _lineBuf.Take(i).ToArray();
                    _lineBuf.RemoveRange(0, i + 1);
                    if (slice.Length > 0 && slice[^1] == (byte)'\r')
                        slice = slice.AsSpan(0, slice.Length - 1).ToArray();
                    return Encoding.UTF8.GetString(slice);
                }
            }

            var n = await pipe.ReadAsync(_readBuf.AsMemory(0, _readBuf.Length), ct).ConfigureAwait(false);
            if (n == 0)
                throw new EndOfStreamException("Core closed the pipe");
            for (var i = 0; i < n; i++)
                _lineBuf.Add(_readBuf[i]);
        }
    }

    public void Dispose()
    {
        Disconnect();
        _gate.Dispose();
    }
}
