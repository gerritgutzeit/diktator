using System.ComponentModel;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
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
        _viewModel.UpdateApplyRequested += () => Dispatcher.Invoke(() => TrayIcon.Unregister());
        _watcher = new DeviceWatcher(this, () =>
        {
            Dispatcher.InvokeAsync(() => _ = _viewModel.HandleDeviceChangeAsync());
        });
        Loaded += OnLoaded;
        Closing += OnClosing;
        DpiChanged += (_, _) => ApplyTrayIcon();
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
        ApplyTrayIcon();
        TrayIcon.Register();
        _watcher.Attach();
        _ = _viewModel.HandleDeviceChangeAsync();
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

    private void ApplyTrayIcon()
    {
        var uri = new Uri("pack://application:,,,/Assets/app.ico", UriKind.Absolute);
        var decoder = BitmapDecoder.Create(uri, BitmapCreateOptions.PreservePixelFormat, BitmapCacheOption.OnLoad);
        var target = 16 * VisualTreeHelper.GetDpi(this).PixelsPerDip;
        TrayIcon.Icon = decoder.Frames
            .OrderBy(frame => Math.Abs(frame.PixelWidth - target))
            .ThenBy(frame => frame.PixelWidth)
            .First();
    }
}
