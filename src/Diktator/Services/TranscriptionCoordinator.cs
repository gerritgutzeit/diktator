namespace Diktator.Services;

internal sealed class TranscriptionCoordinator
{
    private readonly WhisperClient _whisper = new();
    private readonly TranscriptStore _store = new();
    private readonly SemaphoreSlim _gate = new(1, 1);

    public AppSettings Settings { get; private set; } = AppSettings.Load();

    public void ReloadSettings() => Settings = AppSettings.Load();

    public IReadOnlyList<TranscriptEntry> History() => _store.All();

    public string Archive(TranscriptEntry entry)
    {
        var device = PhilipsDetector.Find();
        if (device is null)
            throw new InvalidOperationException("Diktiergerät nicht angeschlossen.");

        var moved = DeviceArchive.MoveToOld(device.Root, entry.FileName);
        _store.Remove(entry.VolumeSerial, entry.FileName, entry.Size);

        return moved
            ? $"{entry.FileName} nach old verschoben"
            : $"{entry.FileName} war nicht mehr im Root, aus Verlauf entfernt";
    }

    public async Task<ScanResult> ScanAndTranscribeAsync(
        bool force,
        IProgress<ScanProgress> progress,
        CancellationToken cancellationToken)
    {
        await _gate.WaitAsync(cancellationToken);
        try
        {
            ReloadSettings();
            progress.Report(new ScanProgress(ScanStatus.Checking, "Suche Diktiergerät…", null, null));

            var device = await WaitForDeviceAsync(cancellationToken);
            if (device is null)
            {
                progress.Report(new ScanProgress(ScanStatus.Waiting, "Warte auf Diktiergerät…", null, null));
                return new ScanResult(false, null, null, null, false);
            }

            var all = RecordingPicker.ListAll(device.Root);
            if (all.Count == 0)
            {
                progress.Report(new ScanProgress(ScanStatus.Device, $"Gerät · {device.Label} · keine Aufnahme", device, null));
                return new ScanResult(false, device, null, null, false);
            }

            var newest = RecordingPicker.PickNewest(device.Root) ?? all[^1];
            var pending = all
                .Where(r => force || _store.Find(device.VolumeSerial, r.FileName, r.Size) is null)
                .ToList();

            if (pending.Count == 0)
            {
                var cached = _store.Find(device.VolumeSerial, newest.FileName, newest.Size);
                progress.Report(new ScanProgress(ScanStatus.Success, $"Gerät · {newest.FileName} · gespeichert", device, newest));
                return new ScanResult(true, device, newest, cached?.Text, true);
            }

            if (string.IsNullOrWhiteSpace(Settings.ApiKey))
            {
                progress.Report(new ScanProgress(ScanStatus.Error, "API-Key fehlt oder ungültig.", device, pending[0]));
                return new ScanResult(false, device, pending[0], null, false, Error: "API-Key fehlt oder ungültig.", OpenSettings: true);
            }

            Recording? lastDone = null;
            string? lastText = null;
            var done = 0;

            try
            {
                for (var i = 0; i < pending.Count; i++)
                {
                    var recording = pending[i];
                    var n = i + 1;
                    progress.Report(new ScanProgress(
                        ScanStatus.Working,
                        pending.Count == 1
                            ? $"Transkribiere {recording.FileName}…"
                            : $"Transkribiere {n}/{pending.Count} · {recording.FileName}",
                        device,
                        recording));

                    var local = await RecordingPicker.CopyToInboxAsync(recording, cancellationToken);
                    var text = await _whisper.TranscribeAsync(local, Settings, cancellationToken);
                    _store.Save(device.VolumeSerial, recording.FileName, recording.Size, text);
                    await TrySummarizeAsync(device.VolumeSerial, recording.FileName, recording.Size, text, cancellationToken);
                    lastDone = recording;
                    lastText = text;
                    done++;
                }
            }
            catch (Exception ex) when (ex is WhisperException or IOException or UnauthorizedAccessException)
            {
                if (done == 0)
                    throw;

                var message = ex is WhisperException we ? we.Message : ex.Message;
                progress.Report(new ScanProgress(ScanStatus.Error, $"{done} fertig, dann Fehler: {message}", device, lastDone));
                return new ScanResult(true, device, lastDone, lastText, false, NewCount: done, Error: message);
            }

            var label = done == 1
                ? $"Gerät · {lastDone!.FileName} · Fertig"
                : $"{done} neu · {lastDone!.FileName} · Fertig";
            progress.Report(new ScanProgress(ScanStatus.Success, label, device, lastDone));
            return new ScanResult(true, device, lastDone, lastText, false, NewCount: done);
        }
        catch (WhisperException ex)
        {
            progress.Report(new ScanProgress(ScanStatus.Error, ex.Message, null, null));
            return new ScanResult(false, null, null, null, false, Error: ex.Message, OpenSettings: ex.StatusCode == 401);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            var message = ex.Message;
            progress.Report(new ScanProgress(ScanStatus.Error, message, null, null));
            return new ScanResult(false, null, null, null, false, Error: message);
        }
        finally
        {
            _gate.Release();
        }
    }

    public Task TestConnectionAsync(CancellationToken cancellationToken)
    {
        ReloadSettings();
        return _whisper.TestConnectionAsync(Settings, cancellationToken);
    }

    public async Task FillMissingSummariesAsync(CancellationToken cancellationToken)
    {
        await _gate.WaitAsync(cancellationToken);
        try
        {
            ReloadSettings();
            if (string.IsNullOrWhiteSpace(Settings.ApiKey))
                return;

            var missing = _store.All()
                .Where(e => string.IsNullOrWhiteSpace(e.Summary) && !string.IsNullOrWhiteSpace(e.Text))
                .ToList();

            foreach (var entry in missing)
            {
                cancellationToken.ThrowIfCancellationRequested();
                await TrySummarizeAsync(entry.VolumeSerial, entry.FileName, entry.Size, entry.Text, cancellationToken);
            }
        }
        finally
        {
            _gate.Release();
        }
    }

    private async Task TrySummarizeAsync(
        string volumeSerial,
        string fileName,
        long size,
        string text,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(text))
            return;

        try
        {
            var summary = await _whisper.SummarizeAsync(text, Settings, cancellationToken);
            if (!string.IsNullOrWhiteSpace(summary))
                _store.SaveSummary(volumeSerial, fileName, size, summary);
        }
        catch
        {
            // Zusammenfassung ist optional.
        }
    }

    private static async Task<PhilipsDevice?> WaitForDeviceAsync(CancellationToken cancellationToken)
    {
        for (var i = 0; i < 6; i++)
        {
            var device = PhilipsDetector.Find();
            if (device is not null)
                return device;
            await Task.Delay(350, cancellationToken);
        }

        return PhilipsDetector.Find();
    }
}

internal enum ScanStatus
{
    Waiting,
    Checking,
    Device,
    Working,
    Success,
    Error
}

internal sealed record ScanProgress(
    ScanStatus Status,
    string Message,
    PhilipsDevice? Device,
    Recording? Recording);

internal sealed record ScanResult(
    bool Ok,
    PhilipsDevice? Device,
    Recording? Recording,
    string? Text,
    bool FromCache,
    int NewCount = 0,
    string? Error = null,
    bool OpenSettings = false);
