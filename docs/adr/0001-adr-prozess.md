# ADR-0001: Entscheidungen als ADRs festhalten

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Diktator ist klein, aber die Regeln (Gerät nicht vollschreiben, Keys nicht committen, alle neuen Dateien transkribieren) sind leicht zu vergessen, wenn nur der Code da ist.

## Entscheidung

Wichtige Architektur- und Produktentscheidungen liegen unter `docs/adr/` als nummerierte Markdown-Dateien. Die README verweist darauf.

## Konsequenzen

- Spätere Änderungen brauchen eine neue ADR oder einen Statuswechsel, nicht nur einen stillen Code-Diff.
- ADRs erklären das *Warum*; Implementierungsdetails bleiben im Code.
