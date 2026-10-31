using Velopack;
using Velopack.Sources;

namespace Diktator.Services;

internal sealed class UpdateCheckResult
{
    public bool IsInstalled { get; init; }
    public bool UpdateAvailable { get; init; }
    public string Message { get; init; } = "";
}

internal static class AppUpdater
{
    public const string RepoUrl = "https://github.com/gerritgutzeit/diktator";

    private static UpdateManager? _manager;
    private static UpdateInfo? _pending;

    public static string DisplayVersion
    {
        get
        {
            try
            {
                var installed = CreateManager().CurrentVersion;
                if (installed is not null)
                    return installed.ToString();
            }
            catch
            {
                // Not a Velopack install (e.g. dotnet run).
            }

            return typeof(AppUpdater).Assembly.GetName().Version?.ToString(3) ?? "1.3.0";
        }
    }

    public static async Task<UpdateCheckResult> CheckAsync(CancellationToken token)
    {
        var mgr = CreateManager();
        if (!mgr.IsInstalled)
        {
            _pending = null;
            return new UpdateCheckResult
            {
                Message = "Updates nur in der installierten App (Setup.exe)."
            };
        }

        token.ThrowIfCancellationRequested();
        var update = await mgr.CheckForUpdatesAsync();
        _manager = mgr;
        _pending = update;
        if (update is null)
        {
            return new UpdateCheckResult
            {
                IsInstalled = true,
                Message = $"Version {DisplayVersion} ist aktuell."
            };
        }

        var version = update.TargetFullRelease.Version.ToString();
        return new UpdateCheckResult
        {
            IsInstalled = true,
            UpdateAvailable = true,
            Message = $"Version {version} ist verfügbar."
        };
    }

    public static async Task ApplyAsync(Action? beforeRestart, Action<int>? progress, CancellationToken token)
    {
        if (_manager is null || _pending is null)
            throw new InvalidOperationException("Zuerst auf Aktualisierung prüfen.");

        await _manager.DownloadUpdatesAsync(_pending, progress, token);
        beforeRestart?.Invoke();
        _manager.ApplyUpdatesAndRestart(_pending);
    }

    private static UpdateManager CreateManager() =>
        new(new GithubSource(RepoUrl, string.Empty, prerelease: false, downloader: null));
}
