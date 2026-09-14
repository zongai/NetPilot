using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using NetPilot.Desktop.ViewModels;

namespace NetPilot.Desktop;

public sealed partial class SubscriptionPage : Page
{
    public SubscriptionViewModel ViewModel { get; } = new();

    public SubscriptionPage()
    {
        InitializeComponent();
        ViewModel.LoadSample();
        SubscriptionList.ItemsSource = ViewModel.Items;
    }

    private void AddButton_Click(object sender, RoutedEventArgs e)
    {
        StatusBar.IsOpen = true;
        StatusBar.Title = "Add subscription";
        StatusBar.Message = "Use Core IPC in a later build; sample list only.";
        StatusBar.Severity = InfoBarSeverity.Informational;
    }

    private void RefreshAllButton_Click(object sender, RoutedEventArgs e)
    {
        ViewModel.LoadSample();
        SubscriptionList.ItemsSource = ViewModel.Items;
        StatusBar.IsOpen = true;
        StatusBar.Title = "Updated";
        StatusBar.Message = "Sample data refreshed (loopback).";
        StatusBar.Severity = InfoBarSeverity.Success;
    }
}
