using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;

namespace NetPilot.Desktop.Wpf;

/// <summary>Newline-delimited JSON IPC client to netpilot-core named pipe.</summary>
/// <remarks>
/// Raw byte I/O + per-request timeout. Disconnect disposes the pipe without
/// synchronously waiting on the request lock (avoids UI-thread deadlock when
/// Refresh/Reconnect runs while ReadAsync is in flight).
/// </remarks>
public sealed class CoreIpcClient : IDisposable
{
    public const string PipeName = "netpilot-core";

    private readonly SemaphoreSlim _gate = new(1, 1);
    private readonly object _pipeLock = new();
    private NamedPipeClientStream? _pipe;
    private readonly byte[] _readBuf = new byte[64 * 1024];
    private readonly List<byte> _lineBuf = new();
    private CancellationTokenSource? _opCts;

    public bool IsConnected
    {
        get
        {
            lock (_pipeLock)
                return _pipe is { IsConnected: true };
        }
    }

    public async Task ConnectAsync(int timeoutMs = 5000, CancellationToken ct = default)
    {
        // Drop any prior pipe first so in-flight reads abort without holding _gate.
        HardClosePipe();

        await _gate.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            HardClosePipe();
            var pipe = new NamedPipeClientStream(
                ".",
                PipeName,
                PipeDirection.InOut,
                PipeOptions.Asynchronous);
            await pipe.ConnectAsync(timeoutMs, ct).ConfigureAwait(false);
            try { pipe.ReadMode = PipeTransmissionMode.Byte; }
            catch { /* optional */ }

            lock (_pipeLock)
            {
                _pipe = pipe;
                _lineBuf.Clear();
            }
        }
        finally
        {
            _gate.Release();
        }
    }

    /// <summary>Abort in-flight I/O and drop the pipe. Safe on UI thread (no sync lock wait).</summary>
    public void Disconnect()
    {
        try { _opCts?.Cancel(); } catch { /* ignore */ }
        HardClosePipe();
    }

    private void HardClosePipe()
    {
        NamedPipeClientStream? pipe;
        lock (_pipeLock)
        {
            pipe = _pipe;
            _pipe = null;
            _lineBuf.Clear();
        }
        if (pipe is null) return;
        try { pipe.Dispose(); } catch { /* ignore */ }
    }

    public async Task<JsonElement> RequestAsync(string operation, JsonElement? payload = null, CancellationToken ct = default)
    {
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(ct);
        linked.CancelAfter(TimeSpan.FromSeconds(8));
        var token = linked.Token;

        // Track op CTS so Disconnect() can cancel this request.
        var prior = Interlocked.Exchange(ref _opCts, linked);
        try { prior?.Dispose(); } catch { /* ignore */ }

        var lockTaken = false;
        try
        {
            await _gate.WaitAsync(token).ConfigureAwait(false);
            lockTaken = true;

            NamedPipeClientStream pipe;
            lock (_pipeLock)
            {
                pipe = _pipe ?? throw new InvalidOperationException("IPC not connected");
                if (!pipe.IsConnected)
                    throw new InvalidOperationException("IPC not connected");
            }

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

            var bytes = Encoding.UTF8.GetBytes(JsonSerializer.Serialize(req) + "\n");
            await pipe.WriteAsync(bytes.AsMemory(0, bytes.Length), token).ConfigureAwait(false);
            await pipe.FlushAsync(token).ConfigureAwait(false);

            for (var i = 0; i < 8; i++)
            {
                token.ThrowIfCancellationRequested();
                var responseLine = await ReadLineAsync(pipe, token).ConfigureAwait(false);
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
            if (lockTaken)
            {
                try { _gate.Release(); } catch { /* ignore */ }
            }
            Interlocked.CompareExchange(ref _opCts, null, linked);
        }
    }

    private async Task<string> ReadLineAsync(NamedPipeClientStream pipe, CancellationToken ct)
    {
        while (true)
        {
            ct.ThrowIfCancellationRequested();

            lock (_pipeLock)
            {
                for (var i = 0; i < _lineBuf.Count; i++)
                {
                    if (_lineBuf[i] != (byte)'\n') continue;
                    var slice = _lineBuf.Take(i).ToArray();
                    _lineBuf.RemoveRange(0, i + 1);
                    if (slice.Length > 0 && slice[^1] == (byte)'\r')
                        slice = slice.AsSpan(0, slice.Length - 1).ToArray();
                    return Encoding.UTF8.GetString(slice);
                }
            }

            int n;
            try
            {
                n = await pipe.ReadAsync(_readBuf.AsMemory(0, _readBuf.Length), ct).ConfigureAwait(false);
            }
            catch (ObjectDisposedException)
            {
                throw new EndOfStreamException("IPC pipe disposed");
            }
            catch (IOException ex)
            {
                throw new EndOfStreamException("IPC pipe IO error: " + ex.Message, ex);
            }

            if (n == 0)
                throw new EndOfStreamException("Core closed the pipe");

            lock (_pipeLock)
            {
                for (var i = 0; i < n; i++)
                    _lineBuf.Add(_readBuf[i]);
            }
        }
    }

    public void Dispose()
    {
        Disconnect();
        try { _gate.Dispose(); } catch { /* ignore */ }
    }
}
