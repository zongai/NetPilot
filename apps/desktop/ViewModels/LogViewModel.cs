// Log streaming UI model (NP-056). Secrets must never appear in lines.

using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace NetPilot.Desktop.ViewModels;

public sealed class LogLine
{
    public string Timestamp { get; init; } = "";
    public string Level { get; init; } = "info";
    public string Message { get; init; } = "";
}

public sealed class LogViewModel : INotifyPropertyChanged
{
    public ObservableCollection<LogLine> Lines { get; } = new();

    private string _filter = "";
    public string Filter
    {
        get => _filter;
        set { _filter = value; OnPropertyChanged(); }
    }

    public void Append(string level, string message)
    {
        // Defense in depth: refuse obvious secret markers.
        if (message.Contains("password=", System.StringComparison.OrdinalIgnoreCase)
            || message.Contains("uuid=", System.StringComparison.OrdinalIgnoreCase))
        {
            message = "[redacted log line]";
        }

        Lines.Add(new LogLine
        {
            Timestamp = System.DateTimeOffset.Now.ToString("HH:mm:ss"),
            Level = level,
            Message = message,
        });
        while (Lines.Count > 500)
            Lines.RemoveAt(0);
    }

    public void LoadSample()
    {
        Lines.Clear();
        Append("info", "Core connected");
        Append("info", "runtime state=running");
        Append("warn", "rule MATCH fallback used");
    }

    public event PropertyChangedEventHandler? PropertyChanged;
    private void OnPropertyChanged([CallerMemberName] string? name = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
