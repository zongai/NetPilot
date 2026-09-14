using System.IO;
using System.IO.Pipes;
using System.Text;
using System.Text.Json;

namespace NetPilot.Desktop.Wpf;

/// <summary>Newline-delimited JSON IPC client to netpilot-core named pipe.</summary>
public sealed class CoreIpcClient : IDisposable
{
    public const string PipeName = "netpilot-core";

    private NamedPipeClientStream? _pipe;
    private StreamReader? _reader;
    private StreamWriter? _writer;

    public bool IsConnected => _pipe is { IsConnected: true };

    public async Task ConnectAsync(int timeoutMs = 5000, CancellationToken ct = default)
    {
        Disconnect();
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

    public void Disconnect()
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
        if (_writer is null || _reader is null || _pipe is null || !_pipe.IsConnected)
            throw new InvalidOperationException("IPC not connected");

        var req = new Dictionary<string, object?>
        {
            ["protocol_version"] = 1u,
            ["kind"] = "request",
            ["request_id"] = Guid.NewGuid().ToString("N"),
            ["operation"] = operation,
        };
        if (payload is { } p)
            req["payload"] = p;

        var line = JsonSerializer.Serialize(req);
        await _writer.WriteLineAsync(line.AsMemory(), ct).ConfigureAwait(false);

        var responseLine = await _reader.ReadLineAsync(ct).ConfigureAwait(false)
            ?? throw new EndOfStreamException("Core closed the pipe");
        using var doc = JsonDocument.Parse(responseLine);
        return doc.RootElement.Clone();
    }

    public void Dispose() => Disconnect();
}
