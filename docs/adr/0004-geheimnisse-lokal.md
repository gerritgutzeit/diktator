# ADR-0004: API-Key nur lokal (DPAPI), nie im Git

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Der Whisper-/Chat-Key darf nicht in Source, README oder GitHub Actions. Die App soll den Key trotzdem über Neustarts behalten.

## Entscheidung

- Speicherort: `%LOCALAPPDATA%\Diktator\settings.json`
- Key im JSON nur als `apiKeyProtected` (DPAPI, `DataProtectionScope.CurrentUser`, Entropy `Diktator.Gutzeit.v1`)
- Klartext-Property `ApiKey` ist `[JsonIgnore]`
- Optionaler Fallback: Umgebungsvariable `DIKTATOR_API_KEY` (wird dann einmalig verschlüsselt gespeichert)
- `.gitignore` schließt `bin/`, `obj/`, `publish/`, `Releases/`, MP3s, lokale Settings aus
- CI bekommt keinen Key; Build und Velopack-Pack brauchen ihn nicht

## Konsequenzen

- Key ist an das Windows-Benutzerkonto gebunden, nicht portabel auf einen anderen PC ohne erneute Eingabe.
- Öffentliches Repo und Releases bleiben secret-frei.
- DPAPI schützt vor casual File-Copy, nicht vor Malware im gleichen User-Kontext.

## Alternativen

- Credential Manager / Windows Hello: mehr UX, gleicher User-Scope.
- Unverschlüsselte settings.json: zu riskant bei Backup-Tools und Sync.
