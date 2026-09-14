// NP-143: Subscription list VM — IPC only; secrets never bound to UI text.
using System.Collections.ObjectModel;
using System.Windows.Input;

namespace NetPilot.Desktop.ViewModels;

public sealed class SubscriptionItemVm
{
    public string Id { get; set; } = "";
    public string Name { get; set; } = "";
    public string UrlDisplay { get; set; } = "";
    public string State { get; set; } = "Idle";
    public string TrafficSummary { get; set; } = "";
    public bool Enabled { get; set; } = true;
    public ICommand? UpdateCommand { get; set; }
}

public sealed class SubscriptionViewModel
{
    public ObservableCollection<SubscriptionItemVm> Items { get; } = new();

    public void LoadSample()
    {
        Items.Clear();
        Items.Add(new SubscriptionItemVm
        {
            Id = "s1",
            Name = "Demo Airport",
            UrlDisplay = "https://example.com/sub",
            State = "Ready",
            TrafficSummary = "used n/a",
            Enabled = true,
        });
    }
}
