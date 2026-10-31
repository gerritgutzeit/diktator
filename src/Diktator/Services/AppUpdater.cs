using Velopack;
using Velopack.Sources;

namespace Diktator.Services;

internal static class AppUpdater
{
    public const string RepoUrl = "https://github.com/gerritgutzeit/diktator";

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

            return typeof(AppUpdater).Assembly.GetName().Version?.ToString(3) ?? "1.2.0";
        }
    }

    public static async Task CheckAsync(Func<bool> isBusy, Action? beforeRestart, Action<string>? status, CancellationToken token)
    {
        try
        {
            var mgr = CreateManager();
            if (!mgr.IsInstalled)
                return;

            await Task.Delay(TimeSpan.FromSeconds(4), token);
            var update = await mgr.CheckForUpdatesAsync();
            if (update is null)
                return;

            status?.Invoke("Update wird geladen…");
            await mgr.DownloadUpdatesAsync(update, cancelToken: token);

            if (isBusy())
            {
                status?.Invoke("Update bereit, wird beim nächsten Start installiert.");
                return;
            }

            beforeRestart?.Invoke();
            mgr.ApplyUpdatesAndRestart(update);
        }
        catch (OperationCanceledException)
        {
        }
        catch
        {
            // Offline or GitHub rate limit — keep the tray app quiet.
        }
    }

    private static UpdateManager CreateManager() =>
        new(new GithubSource(RepoUrl, string.Empty, prerelease: false, downloader: null));
}
