# Architecture Decision Records

Kurze, datierte Entscheidungen: warum die App so gebaut ist, nicht nur wie.

Format angelehnt an [MADR](https://adr.github.io/madr/). Neue Entscheidung: nächste Nummer, Status `Accepted`, bei Änderung die alte ADR auf `Superseded` setzen und verlinken.

| ADR | Titel |
| --- | --- |
| [0001](0001-adr-prozess.md) | Entscheidungen als ADRs festhalten |
| [0002](0002-wpf-tray-app.md) | WPF-Tray-App unter .NET 8 *(superseded by 0013)* |
| [0003](0003-openai-kompatible-endpunkte.md) | OpenAI-kompatible Endpunkte, nichts festverdrahtet |
| [0004](0004-geheimnisse-lokal.md) | API-Key nur lokal (DPAPI), nie im Git |
| [0005](0005-geraet-und-sequenz.md) | DPM4 erkennen, Gutz-Sequenz statt Uhrzeit |
| [0006](0006-alle-neuen-transkribieren.md) | Alle uncached MP3s, Fenster nur bei Neuem |
| [0007](0007-geraet-weitgehend-read-only.md) | Gerät nur lesen; Löschen = `old\` |
| [0008](0008-inbox-kopie.md) | Vor dem Upload nach AppData kopieren |
| [0009](0009-llm-titel-und-kurzfassung.md) | Titel und Kurzfassung per Chat Completions |
| [0010](0010-transkript-ordner.md) | Exportordner mit Datum_Uhrzeit Titel |
| [0011](0011-velopack-github.md) | Velopack von GitHub, Update nur auf Knopf |
| [0012](0012-windows-icons.md) | Mehrgrößen-ICO für die Windows-Shell |
| [0013](0013-rust-slint.md) | Rust + Slint statt WPF/.NET |
