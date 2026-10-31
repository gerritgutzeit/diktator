# ADR-0002: WPF-Tray-App unter .NET 8

- Status: Superseded by [0013](0013-rust-slint.md)
- Datum: 2026-10-31

## Kontext

Die App soll im Hintergrund laufen, ein Philips-USB-Laufwerk mitbekommen und nur bei neuen Diktaten ein dunkles Overlay zeigen. Zielplattform ist Windows 10/11, nicht der Browser.

## Entscheidung

Eine einzelne WPF-App (`net8.0-windows`) mit Infobereich (WPF-UI + Tray):

- Eine Instanz (`Local\Diktator.Gutzeit`); ein zweiter Start weckt nur das Fenster.
- Schließen versteckt das Fenster; Beenden nur über das Tray-Menü.
- Autostart über `HKCU\...\Run`, nicht als Windows-Dienst.
- USB über `WM_DEVICECHANGE` am Fenster-Handle, plus Scan wenn das Gerät schon steckt.

## Konsequenzen

- Kein Dienst, keine UWP-Sandbox, volle Dateisystem- und USB-Rechte des angemeldeten Users.
- UI und Watcher teilen einen Prozess; das Overlay kann sofort Transkripte zeigen.
- Nicht portabel auf macOS/Linux.

## Alternativen

- WinUI 3 / Windows App SDK: neuer, aber schwerer für klassischen Tray und USB.
- Windows-Dienst: kein sinnvolles Overlay, extra IPC.
- Electron: unnötig groß für ein Tray-Tool.
