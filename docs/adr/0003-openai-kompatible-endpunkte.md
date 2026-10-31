# ADR-0003: OpenAI-kompatible Endpunkte, nichts festverdrahtet

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Transkription läuft über einen Whisper-kompatiblen Router, Kurzfassung über Chat Completions. Host und Key sind privat (z. B. Tailscale) und dürfen nicht im öffentlichen Repo stehen.

## Entscheidung

Die App spricht nur OpenAI-ähnliche HTTP-APIs, konfiguriert in den Einstellungen:

- `POST {BASE_URL}/audio/transcriptions` (`model`, `language=de`, Datei als multipart)
- `POST {BASE_URL}/chat/completions` für Titel und Kurzfassung
- `GET …/models` zum Verbindungstest
- Default-Endpunkt im Source ist leer; der User trägt Host und Key lokal ein
- Modell default `auto`, Sprache default `de`

Sibling-URLs leitet `AppSettings` vom Transcriptions-Pfad ab (`/v1/audio/transcriptions` → `/v1/chat/completions`).

## Konsequenzen

- Jeder OpenAI-kompatible Proxy funktioniert, ohne Vendor-SDK.
- Ohne eingetragenen Endpunkt tut die App nichts Sinnvolles — Absicht, damit nichts Privates im Git landet.
- API-Fehler (401 usw.) kommen als JSON im OpenAI-Fehlerformat.

## Alternativen

- Feste URL im Code: bequem lokal, unsicher zum Publizieren.
- Direkte Cloud-SDKs (Azure, OpenAI nuget): Vendor-Lock-in, anderer Auth-Flow.
