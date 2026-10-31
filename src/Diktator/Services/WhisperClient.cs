using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

namespace Diktator.Services;

internal sealed class WhisperException : Exception
{
    public int? StatusCode { get; }
    public string ErrorType { get; }

    public WhisperException(string message, int? statusCode = null, string errorType = "")
        : base(message)
    {
        StatusCode = statusCode;
        ErrorType = errorType;
    }
}

internal sealed class WhisperClient
{
    private static readonly HttpClient Http = CreateClient();

    private static HttpClient CreateClient()
    {
        var client = new HttpClient { Timeout = TimeSpan.FromMinutes(30) };
        client.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
        return client;
    }

    public async Task<string> TranscribeAsync(
        string filePath,
        AppSettings settings,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(settings.ApiKey))
            throw new WhisperException("API-Key fehlt oder ungültig.", 401, "authentication_error");

        await using var stream = File.OpenRead(filePath);
        using var content = new MultipartFormDataContent();
        var fileContent = new StreamContent(stream);
        var extension = Path.GetExtension(filePath).ToLowerInvariant();
        var mediaType = extension == ".wav" ? "audio/wav" : "audio/mpeg";
        fileContent.Headers.ContentType = new MediaTypeHeaderValue(mediaType);
        content.Add(fileContent, "file", Path.GetFileName(filePath));
        content.Add(new StringContent(string.IsNullOrWhiteSpace(settings.Model) ? "auto" : settings.Model), "model");
        if (!string.IsNullOrWhiteSpace(settings.Language))
            content.Add(new StringContent(settings.Language), "language");

        using var request = new HttpRequestMessage(HttpMethod.Post, settings.Endpoint) { Content = content };
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", settings.ApiKey.Trim());

        HttpResponseMessage response;
        try
        {
            response = await Http.SendAsync(request, cancellationToken);
        }
        catch (TaskCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            throw new WhisperException("Zeitüberschreitung beim Whisper-Endpunkt.");
        }
        catch (HttpRequestException)
        {
            throw new WhisperException("NUC nicht erreichbar. Läuft Tailscale?");
        }

        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        if (!response.IsSuccessStatusCode)
            throw ToError((int)response.StatusCode, body);

        return ParseText(body);
    }

    public async Task TestConnectionAsync(AppSettings settings, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(settings.ApiKey))
            throw new WhisperException("API-Key fehlt oder ungültig.", 401, "authentication_error");

        using var request = new HttpRequestMessage(HttpMethod.Get, settings.ModelsUrl());
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", settings.ApiKey.Trim());

        HttpResponseMessage response;
        try
        {
            response = await Http.SendAsync(request, cancellationToken);
        }
        catch (HttpRequestException)
        {
            throw new WhisperException("NUC nicht erreichbar. Läuft Tailscale?");
        }

        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        if (!response.IsSuccessStatusCode)
            throw ToError((int)response.StatusCode, body);
    }

    public async Task<string> SummarizeAsync(
        string transcript,
        AppSettings settings,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(settings.ApiKey))
            throw new WhisperException("API-Key fehlt oder ungültig.", 401, "authentication_error");

        var clipped = transcript.Length > 4000 ? transcript[..4000] : transcript;
        var payload = new
        {
            model = string.IsNullOrWhiteSpace(settings.Model) ? "auto" : settings.Model,
            messages = new object[]
            {
                new
                {
                    role = "system",
                    content = "Fasse das Diktat in einem einzigen kurzen deutschen Satz zusammen (max. etwa 12 Wörter). Kein Vorspann, keine Anführungszeichen."
                },
                new { role = "user", content = clipped }
            }
        };

        using var request = new HttpRequestMessage(HttpMethod.Post, settings.ChatUrl())
        {
            Content = new StringContent(JsonSerializer.Serialize(payload), Encoding.UTF8, "application/json")
        };
        request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", settings.ApiKey.Trim());

        HttpResponseMessage response;
        try
        {
            response = await Http.SendAsync(request, cancellationToken);
        }
        catch (TaskCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            throw new WhisperException("Zeitüberschreitung beim Chat-Endpunkt.");
        }
        catch (HttpRequestException)
        {
            throw new WhisperException("NUC nicht erreichbar. Läuft Tailscale?");
        }

        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        if (!response.IsSuccessStatusCode)
            throw ToError((int)response.StatusCode, body);

        return ParseChat(body);
    }

    private static WhisperException ToError(int status, string body)
    {
        var (message, type) = ParseError(body);
        if (status == 401 || type == "authentication_error")
            return new WhisperException("API-Key fehlt oder ungültig.", 401, "authentication_error");
        if (string.IsNullOrWhiteSpace(message))
            message = $"HTTP {status}";
        return new WhisperException(message, status, type);
    }

    private static (string Message, string Type) ParseError(string body)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            if (doc.RootElement.TryGetProperty("error", out var error))
            {
                var message = error.TryGetProperty("message", out var m) ? m.GetString() ?? "" : "";
                var type = error.TryGetProperty("type", out var t) ? t.GetString() ?? "" : "";
                return (message, type);
            }
        }
        catch
        {
            // Not JSON.
        }

        return (body.Trim(), "");
    }

    private static string ParseText(string body)
    {
        var trimmed = body.Trim();
        if (trimmed.StartsWith('{'))
        {
            try
            {
                using var doc = JsonDocument.Parse(trimmed);
                if (doc.RootElement.TryGetProperty("text", out var text))
                    return (text.GetString() ?? "").Trim();
            }
            catch
            {
                // Fall through to raw body.
            }
        }

        return trimmed;
    }

    private static string ParseChat(string body)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            if (doc.RootElement.TryGetProperty("choices", out var choices) &&
                choices.GetArrayLength() > 0)
            {
                var msg = choices[0].GetProperty("message").GetProperty("content").GetString() ?? "";
                return msg.Trim().Trim('"').Trim();
            }
        }
        catch
        {
            // Fall through.
        }

        return body.Trim();
    }
}
