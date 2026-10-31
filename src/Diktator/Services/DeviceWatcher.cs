using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;

namespace Diktator.Services;

internal sealed class DeviceWatcher : IDisposable
{
    private const int WmDeviceChange = 0x0219;
    private const int DbtDeviceArrival = 0x8000;
    private const int DbtDeviceRemoveComplete = 0x8004;
    private const int DbtDevtypVolume = 0x00000002;

    private readonly Window _window;
    private readonly Action _onChanged;
    private HwndSource? _source;
    private CancellationTokenSource? _debounce;
    private bool _attached;

    public DeviceWatcher(Window window, Action onChanged)
    {
        _window = window;
        _onChanged = onChanged;
    }

    public void Attach()
    {
        if (_attached)
            return;
        _attached = true;
        _window.SourceInitialized += OnSourceInitialized;
        if (PresentationSource.FromVisual(_window) is HwndSource existing)
            Hook(existing);
    }

    public void ScanSoon() => Debounce();

    private void OnSourceInitialized(object? sender, EventArgs e)
    {
        if (PresentationSource.FromVisual(_window) is HwndSource source)
            Hook(source);
    }

    private void Hook(HwndSource source)
    {
        if (_source is not null)
            return;
        _source = source;
        _source.AddHook(WndProc);
        Debounce();
    }

    private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg != WmDeviceChange)
            return IntPtr.Zero;

        var eventType = wParam.ToInt32();
        if (eventType is not (DbtDeviceArrival or DbtDeviceRemoveComplete))
            return IntPtr.Zero;

        if (lParam != IntPtr.Zero)
        {
            var hdr = Marshal.PtrToStructure<DevBroadcastHdr>(lParam);
            if (hdr.DeviceType != DbtDevtypVolume && hdr.DeviceType != 0x00000005)
                return IntPtr.Zero;
        }

        Debounce();
        return IntPtr.Zero;
    }

    private void Debounce()
    {
        _debounce?.Cancel();
        _debounce?.Dispose();
        var cts = new CancellationTokenSource();
        _debounce = cts;
        _ = WaitAndNotify(cts.Token);
    }

    private async Task WaitAndNotify(CancellationToken token)
    {
        try
        {
            await Task.Delay(900, token);
            // Volume may still be mounting.
            for (var i = 0; i < 8 && !token.IsCancellationRequested; i++)
            {
                if (PhilipsDetector.Find() is not null)
                    break;
                await Task.Delay(400, token);
            }
            if (!token.IsCancellationRequested)
                _onChanged();
        }
        catch (TaskCanceledException)
        {
            // Newer event replaced this scan.
        }
    }

    public void Dispose()
    {
        _debounce?.Cancel();
        _debounce?.Dispose();
        if (_source is not null)
        {
            _source.RemoveHook(WndProc);
            _source = null;
        }
        _window.SourceInitialized -= OnSourceInitialized;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct DevBroadcastHdr
    {
        public int Size;
        public int DeviceType;
        public int Reserved;
    }
}
