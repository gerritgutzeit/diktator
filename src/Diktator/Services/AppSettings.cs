using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Diktator.Services;

internal sealed class AppSettings
{
    public const string DefaultEndpoint = "";
    public const string DefaultLanguage = "de";
    public const string DefaultModel = "auto";

    private static readonly byte[] Entropy = "Diktator.Gutzeit.v1"u8.ToArray();
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    public string Endpoint { get; set; } = DefaultEndpoint;
    public string Language { get; set; } = DefaultLanguage;
    public string Model { get; set; } = DefaultModel;
    public bool Autostart { get; set; }
    public string TranscriptFolder { get; set; } = "";

    [JsonIgnore]
    public string ApiKey { get; set; } = "";

    public string? ApiKeyProtected { get; set; }

    public static AppSettings Load()
    {
        AppPaths.EnsureDirectories();
        var settings = new AppSettings();

        if (File.Exists(AppPaths.SettingsFile))
        {
            try
            {
                var json = File.ReadAllText(AppPaths.SettingsFile);
                settings = JsonSerializer.Deserialize<AppSettings>(json, JsonOptions) ?? settings;
                settings.ApiKey = Unprotect(settings.ApiKeyProtected);
            }
            catch
            {
                settings = new AppSettings();
            }
        }

        if (string.IsNullOrWhiteSpace(settings.Endpoint))
            settings.Endpoint = DefaultEndpoint;
        if (string.IsNullOrWhiteSpace(settings.Language))
            settings.Language = DefaultLanguage;
        if (string.IsNullOrWhiteSpace(settings.Model))
            settings.Model = DefaultModel;

        if (string.IsNullOrWhiteSpace(settings.ApiKey))
        {
            var fromEnv = Environment.GetEnvironmentVariable("DIKTATOR_API_KEY");
            if (!string.IsNullOrWhiteSpace(fromEnv))
            {
                settings.ApiKey = fromEnv.Trim();
                settings.Save();
            }
        }

        return settings;
    }

    public void Save()
    {
        AppPaths.EnsureDirectories();
        ApiKeyProtected = string.IsNullOrWhiteSpace(ApiKey) ? null : Protect(ApiKey.Trim());
        var json = JsonSerializer.Serialize(this, JsonOptions);
        File.WriteAllText(AppPaths.SettingsFile, json);
    }

    public string ModelsUrl() => SiblingUrl("models");

    public string ChatUrl() => SiblingUrl("chat/completions");

    private string SiblingUrl(string path)
    {
        var endpoint = Endpoint.TrimEnd('/');
        if (endpoint.Contains("/audio/transcriptions", StringComparison.OrdinalIgnoreCase))
            return endpoint.Replace("/audio/transcriptions", "/" + path, StringComparison.OrdinalIgnoreCase);
        if (endpoint.EndsWith("/v1", StringComparison.OrdinalIgnoreCase))
            return endpoint + "/" + path;
        return endpoint + "/" + path;
    }

    private static string Protect(string plain)
    {
        var bytes = ProtectedData.Protect(Encoding.UTF8.GetBytes(plain), Entropy, DataProtectionScope.CurrentUser);
        return Convert.ToBase64String(bytes);
    }

    private static string Unprotect(string? protectedValue)
    {
        if (string.IsNullOrWhiteSpace(protectedValue))
            return "";
        try
        {
            var bytes = ProtectedData.Unprotect(Convert.FromBase64String(protectedValue), Entropy, DataProtectionScope.CurrentUser);
            return Encoding.UTF8.GetString(bytes);
        }
        catch
        {
            return "";
        }
    }
}
