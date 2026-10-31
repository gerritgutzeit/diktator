using System.IO;
using System.Text.RegularExpressions;

namespace Diktator.Services;

internal sealed record Recording(string Path, string FileName, long Size, DateTime LastWrite, int? Sequence);

internal static class RecordingPicker
{
    private static readonly Regex GutzName = new(@"^Gutz(\d+)\.mp3$", RegexOptions.IgnoreCase | RegexOptions.Compiled);
    private static readonly string[] SkipNames =
    [
        ".Trashes", ".Spotlight-V100", ".fseventsd", "System Volume Information",
        $"{Path.DirectorySeparatorChar}old{Path.DirectorySeparatorChar}",
        $"{Path.AltDirectorySeparatorChar}old{Path.AltDirectorySeparatorChar}"
    ];

    public static IReadOnlyList<Recording> ListAll(string root)
    {
        DirectoryInfo dir;
        try { dir = new DirectoryInfo(root); }
        catch { return []; }

        FileInfo[] files;
        try { files = dir.GetFiles("*.mp3", SearchOption.TopDirectoryOnly); }
        catch { return []; }

        var recordings = files
            .Where(file => !file.Name.StartsWith("._", StringComparison.Ordinal))
            .Where(file => !SkipNames.Any(n => file.FullName.Contains(n, StringComparison.OrdinalIgnoreCase)))
            .Select(ToRecording)
            .Where(r => r is not null)
            .Cast<Recording>()
            .ToList();

        return recordings
            .OrderBy(r => r.Sequence ?? int.MaxValue)
            .ThenBy(r => r.LastWrite)
            .ToList();
    }

    public static Recording? PickNewest(string root)
    {
        var all = ListAll(root);
        if (all.Count == 0)
            return null;

        var sequenced = all.Where(r => r.Sequence is not null).ToList();
        if (sequenced.Count > 0)
            return sequenced.MaxBy(r => r.Sequence);

        var recent = all.Where(r => r.LastWrite.Year >= 2020).ToList();
        return (recent.Count > 0 ? recent : all).MaxBy(r => r.LastWrite);
    }

    public static async Task<string> CopyToInboxAsync(Recording recording, CancellationToken cancellationToken)
    {
        AppPaths.EnsureDirectories();
        var dest = Path.Combine(AppPaths.Inbox, recording.FileName);

        Exception? last = null;
        for (var i = 0; i < 6; i++)
        {
            cancellationToken.ThrowIfCancellationRequested();
            try
            {
                await using var source = new FileStream(
                    recording.Path,
                    FileMode.Open,
                    FileAccess.Read,
                    FileShare.ReadWrite);
                await using var target = new FileStream(dest, FileMode.Create, FileAccess.Write, FileShare.None);
                await source.CopyToAsync(target, cancellationToken);
                return dest;
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                last = ex;
                await Task.Delay(400, cancellationToken);
            }
        }

        throw last ?? new IOException("Datei konnte nicht vom Gerät kopiert werden.");
    }

    private static Recording? ToRecording(FileInfo file)
    {
        try
        {
            var match = GutzName.Match(file.Name);
            int? sequence = match.Success && int.TryParse(match.Groups[1].Value, out var n) ? n : null;
            return new Recording(file.FullName, file.Name, file.Length, file.LastWriteTime, sequence);
        }
        catch
        {
            return null;
        }
    }
}
