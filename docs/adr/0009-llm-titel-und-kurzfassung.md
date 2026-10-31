# ADR-0009: Titel und Kurzfassung per Chat Completions

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Die Verlaufsliste braucht mehr als den Dateinamen. Der Exportordner braucht einen sprechenden Titel. Whisper liefert nur Rohtext.

## Entscheidung

Nach jeder Transkription (und nachträglich für Cache-Einträge ohne Headline) ein Call an `/chat/completions`, `model=auto`:

JSON `{"title":"…","summary":"…"}`

- `title`: 3–6 Wörter, Dateiname-tauglich
- `summary`: ein Satz, ca. 12 Wörter, für die Sidebar

Beides im Cache (`TranscriptEntry.Title` / `.Summary`). Die Liste zeigt die Kurzfassung. Der Titel ist primär für den Dateinamen im Exportordner.

Headline ist **best effort**: Fehler dort dürfen die Transkription nicht kippen.

## Konsequenzen

- Zweiter LLM-Call pro neuer Datei, gleicher Key/Host wie Whisper.
- Freitext statt JSON vom Modell wird als Summary behandelt, Titel fällt auf Summary oder `GutzNNNN` zurück.
- Transkript wird für den Prompt auf 4000 Zeichen gekürzt.

## Alternativen

- Nur Whisper-Text in der Liste: unübersichtlich.
- Lokales Mini-Modell: extra Runtime, anderer Betrieb.
