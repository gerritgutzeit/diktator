using System.ComponentModel;
using System.Windows;
using Diktator.Services;
using Diktator.ViewModels;
using Wpf.Ui.Controls;

namespace Diktator;

public partial class MainWindow
{
    private readonly MainViewModel _viewModel = new();
    private readonly DeviceWatcher _watcher;
    private bool _reallyClose;

    public MainWindow()
    {
        InitializeComponent();
        DataContext = _viewModel;
        _viewModel.ShowWindowRequested += () => Dispatcher.Invoke(Reveal);
        _viewModel.ToastRequested += OnToast;
        _watcher = new DeviceWatcher(this, () =>
        {
            Dispatcher.InvokeAsync(() => _ = _viewModel.HandleDeviceChangeAsync());
        });
        Loaded += OnLoaded;
        Closing += OnClosing;
    }

    public void Reveal()
    {
        Show();
        if (WindowState == WindowState.Minimized)
            WindowState = WindowState.Normal;
        Activate();
        Topmost = true;
        Topmost = false;
        Focus();
    }

    public void RequestExit()
    {
        _reallyClose = true;
        _watcher.Dispose();
        TrayIcon.Unregister();
        Application.Current.Shutdown();
    }

    private void OnLoaded(object sender, RoutedEventArgs e)
    {
        TrayIcon.Register();
        _watcher.Attach();
        _ = _viewModel.HandleDeviceChangeAsync();
        _ = AppUpdater.CheckAsync(
            () => Dispatcher.Invoke(() => _viewModel.IsBusy),
            () => Dispatcher.Invoke(() => TrayIcon.Unregister()),
            msg => Dispatcher.Invoke(() =>
            {
                TrayIcon.TooltipText = msg;
                _viewModel.SettingsMessage = msg;
            }),
            CancellationToken.None);
    }

    private void OnClosing(object? sender, CancelEventArgs e)
    {
        if (_reallyClose)
            return;
        e.Cancel = true;
        Hide();
    }

    private void OnTitleBarClose(TitleBar sender, RoutedEventArgs args)
    {
        args.Handled = true;
        Hide();
    }

    private void OnTrayLeftClick(object sender, RoutedEventArgs e) => Reveal();

    private void OnTrayOpenClick(object sender, RoutedEventArgs e) => Reveal();

    private void OnTraySettingsClick(object sender, RoutedEventArgs e)
    {
        Reveal();
        if (!_viewModel.IsSettingsOpen)
            _viewModel.ToggleSettingsCommand.Execute(null);
    }

    private void OnTrayTranscribeClick(object sender, RoutedEventArgs e)
    {
        Reveal();
        _viewModel.TranscribeNowCommand.Execute(null);
    }

    private void OnTrayExitClick(object sender, RoutedEventArgs e) => RequestExit();

    private void OnToast(string message)
    {
        Dispatcher.Invoke(() => TrayIcon.TooltipText = message);
    }
}
