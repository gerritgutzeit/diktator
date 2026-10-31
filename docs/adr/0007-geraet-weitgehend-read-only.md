# ADR-0007: Gerät nur lesen; Löschen = `old\`

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Das DPM4 ist ein FAT32-Stick. Schreiben kann Aufnahmen gefährden. Trotzdem soll „Löschen“ in der UI die Datei vom aktiven Root nehmen, ohne Windows-Papierkorb auf dem Volume.

## Entscheidung

- Die App schreibt **nicht** ins Volume-Root, außer sie legt `old\` an und verschiebt dorthin.
- UI-Löschen: MP3 (+ optionales `._*`) nach `{root}old\`, Eintrag aus dem Verlauf entfernen.
- `old\` wird beim Scannen ignoriert (wie ein Papierkorb).
- Namenskollision in `old\`: Stem plus Zeitstempel.

## Konsequenzen

- Dateien sind auf dem Gerät noch da, nur unsichtbar für Diktator.
- Kein echtes Wipe, kein Format, keine Firmware-API.
- Andere Software, die das Root liest, sieht die MP3 nach dem Verschieben nicht mehr.

## Alternativen

- Datei wirklich löschen: riskanter auf FAT, kein Undo auf dem Gerät.
- Nur aus dem Cache nehmen: Datei würde beim nächsten Scan wieder transkribiert.
