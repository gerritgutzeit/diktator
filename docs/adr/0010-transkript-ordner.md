# ADR-0010: Exportordner mit Datum_Uhrzeit Titel

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Der JSON-Cache ist für die App, nicht zum Weiterverarbeiten in Explorer, Editor oder Obsidian. Der User will einen Ordner wählen und dort „alles ablegen“.

## Entscheidung

- Optionaler Pfad in den Einstellungen (`TranscriptFolder`), leer = kein Export
- Nach erfolgreicher Transkription (inkl. Headline-Versuch) eine UTF-8-`.txt`
- Name: `yyyy-MM-dd_HH-mm {Titel}.txt` damit Explorer chronologisch sortiert
- Titel aus LLM, sonst Summary, sonst `GutzNNNN`
- „Alles ablegen“ schreibt den gesamten Cache nochmal
- Inhalt: Titel, Originaldateiname, Zeit, Kurzfassung, voller Text

Kein Schreiben auf das Diktiergerät.

## Konsequenzen

- Umbenennen nach späterem Titel erzeugt eine zweite Datei; alte `GutzNNNN.txt` bleiben liegen, bis der User sie löscht.
- Gleicher Titel in derselben Minute überschreibt.
- Der Ordner ist das menschenlesbare Archiv, `transcripts.json` bleibt Source of Truth für „schon transkribiert“.

## Alternativen

- Nur JSON/Markdown-Export auf Knopf: weniger Automatik.
- Dateiname = nur `GutzNNNN`: keine Chronologie, keine Semantik.
