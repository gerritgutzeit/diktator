using System.Text.Json;
using System.Text.Json.Serialization;

namespace Diktator.Services;

internal sealed class TranscriptEntry
{
    public string VolumeSerial { get; set; } = "";
    public string FileName { get; set; } = "";
    public long Size { get; set; }
    public string Text { get; set; } = "";
    public string? Summary { get; set; }
    public DateTime TranscribedAt { get; set; }

    [JsonIgnore]
    public string WhenLabel => TranscribedAt == default ? "" : TranscribedAt.ToString("dd.MM.yy  HH:mm");

    [JsonIgnore]
    public string Preview
    {
        get
        {
            var flat = (Text ?? "").Replace('\n', ' ').Replace('\r', ' ').Trim();
            return flat.Length <= 72 ? flat : flat[..72] + "…";
        }
    }

    [JsonIgnore]
    public string ListSummary
    {
        get
        {
            if (!string.IsNullOrWhiteSpace(Summary))
                return Summary.Trim();
            return string.IsNullOrWhiteSpace(Preview) ? "…" : Preview;
        }
    }

    public bool SameAs(string volumeSerial, string fileName, long size) =>
        string.Equals(VolumeSerial, volumeSerial, StringComparison.OrdinalIgnoreCase) &&
        string.Equals(FileName, fileName, StringComparison.OrdinalIgnoreCase) &&
        Size == size;
}

internal sealed class TranscriptStore
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    private readonly List<TranscriptEntry> _entries;
    private readonly object _gate = new();

    public TranscriptStore()
    {
        _entries = Load();
    }

    public IReadOnlyList<TranscriptEntry> All()
    {
        lock (_gate)
            return _entries.OrderByDescending(e => e.TranscribedAt).ToList();
    }

    public TranscriptEntry? Find(string volumeSerial, string fileName, long size)
    {
        lock (_gate)
        {
            return _entries.FirstOrDefault(e => e.SameAs(volumeSerial, fileName, size));
        }
    }

    public bool Remove(string volumeSerial, string fileName, long size)
    {
        lock (_gate)
        {
            var existing = _entries.FirstOrDefault(e => e.SameAs(volumeSerial, fileName, size));
            if (existing is null)
                return false;
            _entries.Remove(existing);
            Persist();
            return true;
        }
    }

    public TranscriptEntry Save(string volumeSerial, string fileName, long size, string text)
    {
        lock (_gate)
        {
            var existing = _entries.FirstOrDefault(e => e.SameAs(volumeSerial, fileName, size));

            if (existing is null)
            {
                existing = new TranscriptEntry
                {
                    VolumeSerial = volumeSerial,
                    FileName = fileName,
                    Size = size
                };
                _entries.Add(existing);
            }

            if (!string.Equals(existing.Text, text, StringComparison.Ordinal))
                existing.Summary = null;
            existing.Text = text;
            existing.TranscribedAt = DateTime.Now;
            Persist();
            return existing;
        }
    }

    public void SaveSummary(string volumeSerial, string fileName, long size, string summary)
    {
        lock (_gate)
        {
            var existing = _entries.FirstOrDefault(e => e.SameAs(volumeSerial, fileName, size));
            if (existing is null)
                return;
            existing.Summary = summary.Trim();
            Persist();
        }
    }

    private static List<TranscriptEntry> Load()
    {
        AppPaths.EnsureDirectories();
        if (!File.Exists(AppPaths.TranscriptsFile))
            return [];

        try
        {
            var json = File.ReadAllText(AppPaths.TranscriptsFile);
            return JsonSerializer.Deserialize<List<TranscriptEntry>>(json, JsonOptions) ?? [];
        }
        catch
        {
            return [];
        }
    }

    private void Persist()
    {
        AppPaths.EnsureDirectories();
        File.WriteAllText(AppPaths.TranscriptsFile, JsonSerializer.Serialize(_entries, JsonOptions));
    }
}
