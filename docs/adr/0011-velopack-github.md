# ADR-0011: Velopack von GitHub, Update nur auf Knopf

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Die App soll öffentlich gebaut werden, ohne dass User .NET installieren. Auto-Update über GitHub soll gehen, aber nicht mitten in einem Diktat neu starten.

## Entscheidung

- Paketierung: Velopack (`vpk pack`), nicht Single-File-EXE als primäres Release
- GitHub Actions `windows-latest` bei Tag `v*`: `dotnet publish` self-contained win-x64 (Ordner, nicht Single-File), dann `vpk pack` / `vpk upload github`
- Installer-Asset: `Diktator-win-Setup.exe`
- `VelopackApp.Build().Run()` ganz vorn in `Program.Main`
- Updates: **Auf Aktualisierung prüfen**, bei Treffer **Aktualisieren** (Download + Apply + Restart)
- Kein stilles Anwenden beim Start
- `dotnet run` / unpackaged Build: Check erklärt, dass Updates nur in der Setup-Installation gehen
- Alte lose `Diktator.exe` (v1.0.0) kann nicht per Velopack nachziehen — einmal Setup installieren

## Konsequenzen

- Releases brauchen den Tag und den `workflow`-Scope am GitHub-Token.
- Delta-Pakete entstehen, sobald `vpk download github` die vorige Version findet.
- User steuert den Restart; Transkription wird nicht von einem Update getötet, außer sie klicken **Aktualisieren**.

## Alternativen

- Nur Portable-ZIP: kein Update-Mechanismus.
- Stilles Apply-on-start: einfach, aber riskant während Uploads.
- AutoUpdater.NET + lose EXE: weniger Installer-Komfort als Velopack.
