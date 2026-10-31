using System.Threading;
using System.Windows;
using System.Windows.Media;
using Diktator.Services;
using Wpf.Ui.Appearance;

namespace Diktator;

public partial class App : Application
{
    private const string MutexName = @"Local\Diktator.Gutzeit";
    private const string ShowEventName = @"Local\Diktator.Gutzeit.Show";

    private Mutex? _mutex;
    private EventWaitHandle? _showEvent;
    private CancellationTokenSource? _showListener;
    private MainWindow? _window;

    protected override void OnStartup(StartupEventArgs e)
    {
        _mutex = new Mutex(true, MutexName, out var created);
        if (!created)
        {
            try
            {
                EventWaitHandle.OpenExisting(ShowEventName).Set();
            }
            catch
            {
                // First instance may already be gone.
            }

            Shutdown();
            return;
        }

        _showEvent = new EventWaitHandle(false, EventResetMode.AutoReset, ShowEventName);
        _showListener = new CancellationTokenSource();
        _ = ListenForShowAsync(_showListener.Token);

        ApplicationThemeManager.Apply(ApplicationTheme.Dark);
        ApplicationAccentColorManager.Apply(Color.FromRgb(0x6D, 0x5E, 0xF6), ApplicationTheme.Dark);

        AppPaths.EnsureDirectories();
        var settings = AppSettings.Load();
        if (settings.Autostart)
            AutostartService.SetEnabled(true);

        _window = new MainWindow();
        _window.Show();
        if (string.IsNullOrWhiteSpace(settings.ApiKey))
            _window.Reveal();
        else
            _window.Hide();

        base.OnStartup(e);
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _showListener?.Cancel();
        _showEvent?.Dispose();
        _mutex?.ReleaseMutex();
        _mutex?.Dispose();
        base.OnExit(e);
    }

    private async Task ListenForShowAsync(CancellationToken token)
    {
        await Task.Yield();
        while (!token.IsCancellationRequested)
        {
            var signaled = await Task.Run(() =>
            {
                try { return _showEvent?.WaitOne(500) == true; }
                catch { return false; }
            }, token);

            if (signaled)
                Dispatcher.Invoke(() => _window?.Reveal());
        }
    }
}
