# ADR-0012: Mehrgrößen-ICO für die Windows-Shell

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Eine einzelne 1024px-PNG als `Window.Icon` / Tray skaliert Windows unscharf (Taskleiste, Alt-Tab, NotifyIcon, 125–200 % DPI).

## Entscheidung

- Native `ApplicationIcon`: `Assets/app.ico` mit 16, 20, 24, 32, 40, 48, 64, 128, 256 (Area-Downscale vom App-PNG)
- Fenster-Icon nicht mit der Riesen-PNG überschreiben — die EXE-ICO liefert alle Größen
- Tray: zur Laufzeit den ICO-Frame wählen, der `16 × DPI` am nächsten ist
- Titelleiste: `app-48.png`
- Velopack-Pack bekommt dieselbe ICO (`-i`)

## Konsequenzen

- Scharfe Shell-Icons auf 100–200 % DPI, solange die ICO neu gebaut wird, wenn sich das Master-PNG ändert.
- Tray hängt an WPF-UI `ImageSource`; deshalb Frame-Picking statt „einfach die PNG“.

## Alternativen

- Nur SVG zur Laufzeit: WPF hat keinen nativen SVG-Decoder.
- Eine 256px-PNG überall: auf 16px weiterhin matschig.
