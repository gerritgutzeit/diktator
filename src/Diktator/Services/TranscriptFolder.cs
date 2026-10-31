using System.Text;

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
        var stem = Path.GetFileNameWithoutExtension(entry.FileName);
        if (string.IsNullOrWhiteSpace(stem))
            stem = "transkript";
        foreach (var c in Path.GetInvalidFileNameChars())
            stem = stem.Replace(c, '_');
        return stem + ".txt";
    }

    private static string Format(TranscriptEntry entry)
    {
        var sb = new StringBuilder();
        if (!string.IsNullOrWhiteSpace(entry.FileName))
            sb.AppendLine(entry.FileName);
        if (entry.TranscribedAt != default)
            sb.AppendLine(entry.TranscribedAt.ToString("dd.MM.yyyy HH:mm"));
        if (!string.IsNullOrWhiteSpace(entry.Summary))
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
