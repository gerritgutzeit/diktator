# ADR-0005: DPM4 erkennen, Gutz-Sequenz statt Uhrzeit

- Status: Accepted
- Datum: 2026-10-31

## Kontext

Das Philips Pocket Memo DPM4 hängt als FAT32-Volume. Die RTC setzt sich oft zurück, `LastWriteTime` ist unzuverlässig. Dateien heißen `GutzNNNN.MP3` in der Volume-Wurzel.

## Entscheidung

Gerät erkennen über:

1. Wechseldatenträger mit Volume-Label `PHILIPS`, oder
2. USB `VID_0911` / Hinweis DPM4 bzw. PHILIPS in WMI (`PNPDeviceID` / Model)

Aufnahmen nur im Root, `*.mp3`, keine AppleDouble (`._*`), nicht unter `old\`, `.Trashes`, usw.

Reihenfolge und „neueste Datei“ über die Zahl in `Gutz(\d+)`. `LastWriteTime` nur Fallback, und Jahre vor 2020 werden als kaputte Uhr ignoriert.

Volume-Serial plus Dateiname plus Größe identifizieren eine Aufnahme über Steckzyklen hinweg.

## Konsequenzen

- Robust gegen RTC-Reset und gegen Umstecken mit wechselndem Laufwerksbuchstaben.
- Andere Diktiergeräte ohne `GutzNNNN` / Philips-USB werden nicht unterstützt.
- WMI kann fehlschlagen; Label `PHILIPS` reicht dann.

## Alternativen

- Nur Timestamp: falsch bei Uhren-Reset.
- Fester Laufwerksbuchstabe: bricht, sobald Windows einen anderen Buchstaben vergibt.
