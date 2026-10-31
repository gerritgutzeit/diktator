# ADR-0008: Vor dem Upload nach AppData kopieren

- Status: Accepted
- Datum: 2026-10-31

## Kontext

USB-FAT kann mitten im Read wegschnappen (Sleep, Abziehen, Exclusive Lock der Firmware). Ein direkter Stream vom Stick in den HTTP-Body ist brüchig.

## Entscheidung

Jede Aufnahme wird zuerst nach `%LOCALAPPDATA%\Diktator\inbox\` kopiert (`FileShare.ReadWrite`, ein paar Retries) und erst dann an `/audio/transcriptions` geschickt.

## Konsequenzen

- Kurzer Extra-Disk-Verbrauch; Inbox ist Arbeitskopie, nicht Archiv (Archiv ist Cache + optionaler Transkript-Ordner).
- Transkription überlebt, wenn das Gerät während des Uploads verschwindet — sofern die Kopie schon da war.

## Alternativen

- Direkter Upload vom Stick: weniger IO, mehr Abbruch.
- RAM-Buffer ganzer Dateien: schlecht bei langen Diktaten.
