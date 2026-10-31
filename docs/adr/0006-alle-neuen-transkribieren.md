# ADR-0006: Alle uncached MP3s, Fenster nur bei Neuem

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Mehrere neue Diktate können auf dem Gerät liegen. Nur die neueste Datei zu schicken würde ältere Lücken lassen. Schon transkribierte Dateien dürfen nicht nochmal hochgeladen werden, und das Overlay soll nicht bei jedem USB-Event aufpoppen.

## Entscheidung

- Cache: `%LOCALAPPDATA%\Diktator\transcripts.json`, Schlüssel `volumeSerial + fileName + size`
- Beim Anstecken oder „Jetzt transkribieren“: alle Root-MP3s, die nicht im Cache sind, **älteste zuerst** (Sequenz)
- Zwischenablage = Text der **neuesten** fertigen Datei
- Fenster nur öffnen, wenn `NewCount > 0` (oder Fehler / fehlender Key)
- Verlauf in der Sidebar aus dem Cache, unabhängig vom Gerät

## Konsequenzen

- Erstanschluss mit vielen alten, nie gecachten MP3s kann lange dauern und große Uploads auslösen — gewollt, einmalig.
- Größenänderung derselben `GutzNNNN.MP3` gilt als neue Aufnahme.
- Umbenennen auf dem Gerät erzeugt einen neuen Cache-Eintrag.

## Alternativen

- Nur neueste Datei: weniger Last, verlorene Diktate.
- Fenster immer: störend für Tray-Alltag.
