using System.Text;
using System.Text.RegularExpressions;

namespace Diktator.Services;

internal static class TranscriptFolder
{
    public static bool IsSet(string? folder) => !string.IsNullOrWhiteSpace(folder);

    public static bool TryWrite(string? folder, TranscriptEntry entry, out string? error)
    {
        error = null;
        if (!IsSet(folder) || string.IsNullOrWhiteSpace(entry.Text))
            return false;

        try
        {
            Directory.CreateDirectory(folder!);
            var path = Path.Combine(folder!, FileNameFor(entry));
            File.WriteAllText(path, Format(entry), Encoding.UTF8);
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    public static int WriteAll(string? folder, IEnumerable<TranscriptEntry> entries, out string? error)
    {
        error = null;
        if (!IsSet(folder))
        {
            error = "Kein Transkript-Ordner gesetzt.";
            return 0;
        }

        var count = 0;
        foreach (var entry in entries)
        {
            if (TryWrite(folder, entry, out var writeError))
            {
                count++;
                continue;
            }

            if (!string.IsNullOrEmpty(writeError))
            {
                error = writeError;
                return count;
            }
        }

        return count;
    }

    public static string FileNameFor(TranscriptEntry entry)
    {
        var when = entry.TranscribedAt == default ? DateTime.Now : entry.TranscribedAt;
        var stamp = when.ToString("yyyy-MM-dd_HH-mm");
        var title = SanitizeTitle(entry.Title);
        if (string.IsNullOrWhiteSpace(title))
            title = SanitizeTitle(entry.Summary);
        if (string.IsNullOrWhiteSpace(title))
            title = SanitizeTitle(Path.GetFileNameWithoutExtension(entry.FileName));
        if (string.IsNullOrWhiteSpace(title))
            title = "Transkript";
        return $"{stamp} {title}.txt";
    }

    internal static string SanitizeTitle(string? raw)
    {
        if (string.IsNullOrWhiteSpace(raw))
            return "";

        var text = raw.Trim().Trim('"').Trim('\'');
        text = text.Replace(':', ' ').Replace('/', '-').Replace('\\', '-');
        foreach (var c in Path.GetInvalidFileNameChars())
            text = text.Replace(c, ' ');
        text = Regex.Replace(text, @"\s+", " ").Trim().TrimEnd('.');
        if (text.Length > 80)
        {
            var cut = text[..80];
            var space = cut.LastIndexOf(' ');
            text = space >= 24 ? cut[..space] : cut;
        }

        return text;
    }

    private static string Format(TranscriptEntry entry)
    {
        var sb = new StringBuilder();
        var title = string.IsNullOrWhiteSpace(entry.Title) ? entry.Summary : entry.Title;
        if (!string.IsNullOrWhiteSpace(title))
            sb.AppendLine(title.Trim());
        if (!string.IsNullOrWhiteSpace(entry.FileName))
            sb.AppendLine(entry.FileName);
        if (entry.TranscribedAt != default)
            sb.AppendLine(entry.TranscribedAt.ToString("dd.MM.yyyy HH:mm"));
        if (!string.IsNullOrWhiteSpace(entry.Summary) &&
            !string.Equals(entry.Summary.Trim(), title?.Trim(), StringComparison.Ordinal))
        {
            sb.AppendLine();
            sb.AppendLine(entry.Summary.Trim());
        }

        sb.AppendLine();
        sb.Append(entry.Text.Trim());
        if (!entry.Text.EndsWith('\n'))
            sb.AppendLine();
        return sb.ToString();
    }
}
