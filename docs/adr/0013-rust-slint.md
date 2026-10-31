# ADR-0013: Rust + Slint statt WPF/.NET

- Status: Accepted
- Datum: 2026-10-31
- Supersedes: [0002](0002-wpf-tray-app.md)

## Kontext

Die App war eine WPF-Tray-Anwendung unter .NET 8 (ADR-0002). Eine Neuimplementierung soll denselben Funktionsumfang und dieselben lokalen Daten behalten, aber ohne .NET-Runtime und ohne Mica/Fluent-Abhängigkeit. Zielplattform bleibt Windows 10/11.

## Entscheidung

Ein Windows-only Rust-Binary im Cargo-Workspace (`crates/diktator`) mit Slint-UI:

- UI-Thread ist Slint (dunkles Overlay, Farben wie bisher: Surface `#1C1C1F`, Accent `#6D5EF6`).
- Transkription, HTTP und USB-Ereignisse laufen in Tokio; Rückweg in die UI über `slint::invoke_from_event_loop`.
- Eine Instanz (`Local\Diktator.Gutzeit`); ein zweiter Start setzt `Local\Diktator.Gutzeit.Show` und beendet sich.
- Schließen versteckt das Fenster; Beenden nur über das Tray-Menü.
- Autostart über `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- USB über ein Message-only-HWND (`WM_DEVICECHANGE`), plus Scan wenn das Gerät schon steckt.
- Bestehende AppData-Daten unter `%LOCALAPPDATA%\Diktator\` bleiben, inkl. DPAPI mit Entropy `Diktator.Gutzeit.v1`.
- Velopack bleibt der Updater (gleiche GitHub-Releases, packId `Diktator`). `cargo run` ohne Velopack-Install zeigt denselben Hinweis wie zuvor `dotnet run`.
- Kein Mica: Slint kann das nicht sinnvoll nachbauen; das dunkle runde Overlay reicht.

Das C#-Projekt bleibt parallel liegen, bis die Rust-Version denselben Funktionsumfang hat. Ein späteres Rust-Release nutzt Major **2.0.0**, damit C#-1.3-User über Velopack umsteigen können.

Die übrigen ADRs (Endpunkte, Secrets, Gerät, Cache, `old\`, Inbox, LLM, Exportordner, Velopack, ICO) gelten unverändert.

## Konsequenzen

- Kein Dienst, volle Dateisystem- und USB-Rechte des angemeldeten Users.
- Build braucht die MSVC-Toolchain und das Windows SDK (`cargo build -p diktator`).
- Nicht portabel auf macOS/Linux.
- C# und Rust teilen sich dieselben lokalen Dateien; ein paralleler Start beider Binaries ist weiterhin durch den Named Mutex ausgeschlossen.

## Alternativen

- egui: schneller zum Prototyp, weniger natives Tray/Window-Feeling.
- Tauri: Webview und extra Runtime für ein Tray-Tool unnötig groß.
- WPF behalten (ADR-0002): funktioniert, bindet aber weiter an .NET.
