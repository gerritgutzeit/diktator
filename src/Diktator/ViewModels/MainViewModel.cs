using System.Collections.ObjectModel;
using System.Windows;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Diktator.Services;

namespace Diktator.ViewModels;

internal partial class MainViewModel : ObservableObject
{
    private readonly TranscriptionCoordinator _coordinator = new();
    private CancellationTokenSource? _runCts;
    private bool _copyFlashResetQueued;
    private bool _suppressSelection;

    [ObservableProperty] private string _statusText = "Warte auf Diktiergerät…";
    [ObservableProperty] private ScanStatus _status = ScanStatus.Waiting;
    [ObservableProperty] private string _transcript = "";
    [ObservableProperty] private bool _isBusy;
    [ObservableProperty] private bool _isSettingsOpen;
    [ObservableProperty] private bool _isHistoryOpen;
    [ObservableProperty] private bool _hasTranscript;
    [ObservableProperty] private bool _hasHistory;
    [ObservableProperty] private string _copyButtonText = "Kopieren";
    [ObservableProperty] private string _fileHint = "";
    [ObservableProperty] private string _errorText = "";
    [ObservableProperty] private bool _hasError;
    [ObservableProperty] private TranscriptEntry? _selectedHistory;

    [ObservableProperty] private string _settingsEndpoint = AppSettings.DefaultEndpoint;
    [ObservableProperty] private string _settingsApiKey = "";
    [ObservableProperty] private string _settingsLanguage = AppSettings.DefaultLanguage;
    [ObservableProperty] private bool _settingsAutostart;
    [ObservableProperty] private string _settingsMessage = "";

    public string AppVersion { get; } = AppUpdater.DisplayVersion;

    public ObservableCollection<TranscriptEntry> History { get; } = [];

    public event Action<string>? ToastRequested;
    public event Action? ShowWindowRequested;

    public MainViewModel()
    {
        LoadSettingsIntoForm();
        RefreshHistory();
        if (string.IsNullOrWhiteSpace(_coordinator.Settings.ApiKey))
            IsSettingsOpen = true;
        _ = FillSummariesAsync();
    }

    public async Task HandleDeviceChangeAsync()
    {
        await RunAsync(showWindowOnNew: true);
        await FillSummariesAsync();
    }

    [RelayCommand]
    private async Task TranscribeNowAsync()
    {
        await RunAsync(showWindowOnNew: true);
        await FillSummariesAsync();
    }

    [RelayCommand]
    private void ToggleHistory() => IsHistoryOpen = !IsHistoryOpen;

    [RelayCommand]
    private void Copy()
    {
        if (string.IsNullOrWhiteSpace(Transcript))
            return;
        TrySetClipboard(Transcript);
        FlashCopied();
    }

    [RelayCommand]
    private void Delete(TranscriptEntry? entry)
    {
        entry ??= SelectedHistory;
        if (entry is null)
            return;

        try
        {
            var message = _coordinator.Archive(entry);
            RefreshHistory();
            HasError = false;
            ErrorText = "";
            StatusText = message;
            Status = ScanStatus.Success;

            var next = History.FirstOrDefault();
            if (next is null)
            {
                SelectedHistory = null;
                Transcript = "";
                HasTranscript = false;
                FileHint = "";
            }
            else
            {
                SelectedHistory = next;
                StatusText = message;
            }

            ToastRequested?.Invoke(message);
        }
        catch (Exception ex)
        {
            HasError = true;
            ErrorText = ex.Message;
            Status = ScanStatus.Error;
            StatusText = ex.Message;
        }
    }

    [RelayCommand]
    private void ToggleSettings()
    {
        IsSettingsOpen = !IsSettingsOpen;
        if (IsSettingsOpen)
            LoadSettingsIntoForm();
        SettingsMessage = "";
    }

    [RelayCommand]
    private void SaveSettings()
    {
        var settings = AppSettings.Load();
        settings.Endpoint = string.IsNullOrWhiteSpace(SettingsEndpoint) ? AppSettings.DefaultEndpoint : SettingsEndpoint.Trim();
        settings.ApiKey = SettingsApiKey.Trim();
        settings.Language = string.IsNullOrWhiteSpace(SettingsLanguage) ? AppSettings.DefaultLanguage : SettingsLanguage.Trim();
        settings.Autostart = SettingsAutostart;
        settings.Save();
        AutostartService.SetEnabled(SettingsAutostart);
        _coordinator.ReloadSettings();
        SettingsMessage = "Gespeichert.";
        HasError = false;
        ErrorText = "";
    }

    [RelayCommand]
    private async Task TestConnectionAsync()
    {
        SaveSettings();
        SettingsMessage = "Prüfe Verbindung…";
        try
        {
            using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(20));
            await _coordinator.TestConnectionAsync(cts.Token);
            SettingsMessage = "Verbindung ok.";
        }
        catch (WhisperException ex)
        {
            SettingsMessage = ex.Message;
        }
        catch (Exception ex)
        {
            SettingsMessage = ex.Message;
        }
    }

    private void LoadSettingsIntoForm()
    {
        var settings = AppSettings.Load();
        SettingsEndpoint = settings.Endpoint;
        SettingsApiKey = settings.ApiKey;
        SettingsLanguage = settings.Language;
        SettingsAutostart = settings.Autostart || AutostartService.IsEnabled();
    }

    private async Task RunAsync(bool showWindowOnNew)
    {
        _runCts?.Cancel();
        _runCts = new CancellationTokenSource();
        var token = _runCts.Token;

        IsBusy = true;
        HasError = false;
        ErrorText = "";

        var progress = new Progress<ScanProgress>(p =>
        {
            Status = p.Status;
            StatusText = p.Message;
            FileHint = p.Recording?.FileName ?? "";
            IsBusy = p.Status is ScanStatus.Working or ScanStatus.Checking;
        });

        try
        {
            var result = await _coordinator.ScanAndTranscribeAsync(force: false, progress, token);
            if (token.IsCancellationRequested)
                return;

            if (result.OpenSettings)
            {
                IsSettingsOpen = true;
                ShowWindowRequested?.Invoke();
            }

            if (!result.Ok && result.NewCount == 0)
            {
                if (!string.IsNullOrEmpty(result.Error))
                {
                    HasError = true;
                    ErrorText = result.Error;
                    Status = ScanStatus.Error;
                    StatusText = result.Error;
                    ShowWindowRequested?.Invoke();
                    return;
                }

                if (result.Device is null)
                {
                    Status = ScanStatus.Waiting;
                    StatusText = "Warte auf Diktiergerät…";
                    FileHint = "";
                }
                return;
            }

            RefreshHistory();
            SelectHistory(result.Recording);

            if (!string.IsNullOrWhiteSpace(result.Text))
            {
                Transcript = result.Text;
                HasTranscript = true;
            }

            if (!string.IsNullOrEmpty(result.Error))
            {
                HasError = true;
                ErrorText = result.Error;
            }

            if (result.FromCache || result.NewCount == 0)
            {
                ToastRequested?.Invoke($"{result.Recording?.FileName} bereits transkribiert");
                return;
            }

            TrySetClipboard(result.Text ?? "");
            FlashCopied();
            IsHistoryOpen = true;
            if (showWindowOnNew)
                ShowWindowRequested?.Invoke();

            var toast = result.NewCount == 1
                ? $"Transkript von {result.Recording?.FileName}"
                : $"{result.NewCount} neue Transkripte · zuletzt {result.Recording?.FileName}";
            ToastRequested?.Invoke(toast);
        }
        catch (OperationCanceledException)
        {
            // Replaced by a newer scan.
        }
        finally
        {
            IsBusy = false;
        }
    }

    partial void OnSelectedHistoryChanged(TranscriptEntry? value)
    {
        if (_suppressSelection || value is null)
            return;
        Transcript = value.Text;
        HasTranscript = !string.IsNullOrWhiteSpace(value.Text);
        FileHint = value.FileName;
        StatusText = $"{value.FileName} · {value.WhenLabel}";
        Status = ScanStatus.Success;
        HasError = false;
        IsSettingsOpen = false;
    }

    private void RefreshHistory()
    {
        var selectedKey = SelectedHistory;
        var items = _coordinator.History();
        _suppressSelection = true;
        History.Clear();
        foreach (var item in items)
            History.Add(item);
        HasHistory = History.Count > 0;
        if (selectedKey is not null)
        {
            SelectedHistory = History.FirstOrDefault(h =>
                h.SameAs(selectedKey.VolumeSerial, selectedKey.FileName, selectedKey.Size));
        }
        _suppressSelection = false;
    }

    private async Task FillSummariesAsync()
    {
        try
        {
            await _coordinator.FillMissingSummariesAsync(CancellationToken.None);
        }
        catch
        {
            return;
        }

        var keep = SelectedHistory;
        RefreshHistory();
        if (keep is not null)
        {
            SelectedHistory = History.FirstOrDefault(h =>
                h.SameAs(keep.VolumeSerial, keep.FileName, keep.Size)) ?? SelectedHistory;
        }
    }

    private void SelectHistory(Recording? recording)
    {
        if (recording is null)
            return;
        _suppressSelection = true;
        SelectedHistory = History.FirstOrDefault(h =>
            string.Equals(h.FileName, recording.FileName, StringComparison.OrdinalIgnoreCase) &&
            h.Size == recording.Size) ?? History.FirstOrDefault();
        _suppressSelection = false;
    }

    private void FlashCopied()
    {
        CopyButtonText = "Kopiert";
        if (_copyFlashResetQueued)
            return;
        _copyFlashResetQueued = true;
        _ = ResetCopyLabelAsync();
    }

    private async Task ResetCopyLabelAsync()
    {
        await Task.Delay(1800);
        CopyButtonText = "Kopieren";
        _copyFlashResetQueued = false;
    }

    private static void TrySetClipboard(string text)
    {
        try
        {
            Clipboard.SetText(text);
        }
        catch
        {
            try { Clipboard.SetDataObject(text, true); }
            catch { /* Clipboard can be locked. */ }
        }
    }
}
