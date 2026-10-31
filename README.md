# Diktator

<p align="center">
  <img src="src/Diktator/Assets/diktator-github-repo.png" alt="Diktator – Deine Stimme. Dein Text." width="840" />
</p>

Hintergrund-App für Windows: erkennt ein Philips Pocket Memo (DPM4), sobald es per USB als Laufwerk erscheint, schickt **alle noch nicht transkribierten MP3s** an einen OpenAI-kompatiblen Whisper-Endpunkt und legt den neuesten Text in die Zwischenablage.

Läuft im Infobereich. Dunkles Overlay mit Transkript, Verlauf, Kurzfassung und Kopieren-Button.

## Voraussetzungen

- Windows 10/11
- OpenAI-kompatibler Endpunkt:
  - `POST {BASE_URL}/audio/transcriptions` (Whisper)
  - `POST {BASE_URL}/chat/completions` (Kurzfassung)
- Philips DPM4, Volume-Label `PHILIPS`

Der API-Key und der Endpunkt gehören **nur in die App-Einstellungen** (lokal per DPAPI verschlüsselt) oder optional in die Umgebungsvariable `DIKTATOR_API_KEY`. Beides kommt nicht ins Git-Repo.

## Dokumentation

Architekturentscheidungen (Warum, nicht nur Wie): [docs/adr](docs/adr/README.md).

## Starten (Entwicklung)

Rust (aktuell, Slint, `crates/diktator`):

```powershell
cargo run -p diktator
```

MSVC-Buildtools und das Windows SDK müssen installiert sein. Dieselben Dateien unter `%LOCALAPPDATA%\Diktator\` wie bisher.

C# (WPF, parallel bis 1.3):

```powershell
dotnet run --project src/Diktator
```

Beim ersten Start unter dem Zahnrad **Endpunkt** und **API-Key** eintragen, **Speichern**, optional **Verbindung testen**.

Beispiel-Endpunkt: `https://<dein-host>/v1/audio/transcriptions`

## Release (GitHub)

Unter [Releases](https://github.com/gerritgutzeit/diktator/releases) liegt `Diktator-win-Setup.exe`. Ab **v2.0.0** ist das die Rust-App. Einmal installieren; neue Versionen prüfst du unter Einstellungen mit **Auf Aktualisierung prüfen**.

Jeder Tag `v*` (z. B. `v2.0.0`) baut auf `windows-latest` den Installer und die Update-Pakete. Velopack-Id bleibt `Diktator`, damit 1.3-Installationen auf 2.0 aktualisieren können.

Manuell:

```powershell
cargo build --release -p diktator
New-Item -ItemType Directory -Force publish | Out-Null
Copy-Item target\release\diktator.exe publish\Diktator.exe
vpk pack -u Diktator -v 2.0.0 -p publish --mainExe Diktator.exe -i src/Diktator/Assets/app.ico
```

## Verhalten

- Gerät schon steckt oder neu eingesteckt → alle neuen `GutzNNNN.MP3` der Reihe nach
- Schon transkribiert → kein zweiter Upload, Fenster bleibt zu
- Mehrere neue → alle werden transkribiert, Fenster öffnet sich, Verlauf listet sie, Zwischenablage = die neueste
- Datei wird nach `%LOCALAPPDATA%\Diktator\inbox\` kopiert
- **Löschen** verschiebt die MP3 auf dem Gerät nach `old\` (Papierkorb, wird ignoriert)
- Optionaler **Transkript-Ordner**: `.txt` als `Datum_Uhrzeit Titel.txt` (Titel aus der Kurzfassung)
- API-Key liegt per DPAPI in `%LOCALAPPDATA%\Diktator\settings.json`
- Schließen legt die App in den Tray; Beenden nur über das Tray-Menü
