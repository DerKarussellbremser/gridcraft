# Anleitung: gridcraft-de auf dem Mac holen und als Universal-DMG bauen

Vom ersten `git clone` (oder einem späteren `git pull`) bis zur fertigen Installations-DMG, die auf
Macs mit Apple Silicon (M1, M2 …) und mit Intel-Prozessor läuft. Alle Befehle gehören ins Terminal
(Programme › Dienstprogramme › Terminal).

## 1. Einmalig: Werkzeuge einrichten

```sh
# Apples Kommandozeilen-Werkzeuge (git, lipo, codesign); öffnet ein Fenster zum Bestätigen
xcode-select --install

# Rust (Antworten mit der Eingabetaste übernehmen), danach ein neues Terminal öffnen
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Beide Mac-Prozessoren als Ziel, für die Universal-DMG
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

Ist Rust schon da: `rustup update` (gridcraft braucht Rust 1.90 oder neuer).

## 2. Den Stand holen

**Noch nie geholt?**

```sh
git clone -b german-locale https://github.com/DerKarussellbremser/gridcraft-de.git ~/gridcraft-de
cd ~/gridcraft-de
```

**Schon einmal geholt?** Auf den neuesten Stand bringen:

```sh
cd ~/gridcraft-de
git checkout german-locale
git pull
```

Meldet `git pull`, lokale Änderungen stünden im Weg (es wird hier nichts von Hand geändert):
`git status` zeigt sie; `git restore .` verwirft sie, dann `git pull` noch einmal.

## 3. Die Universal-DMG bauen

```sh
cd ~/gridcraft-de
packaging/macos/package.sh
```

Das Skript baut gridcraft zweimal (Apple Silicon und Intel), fügt beides mit `lipo` zu einem
Programm zusammen, setzt `GridCraft.app` zusammen, signiert es und packt es mit Hintergrundbild und
`Programme`-Verknüpfung in eine DMG. Der erste Lauf dauert eine ganze Weile (je nach Mac 10 bis 30
Minuten), spätere Läufe gehen schneller.

Am Ende steht `==> done`, und die Ergebnisse liegen in `dist/release/`:

| Datei | Was |
|---|---|
| `gridcraft-<Version>-macos-universal.dmg` | die Installations-DMG |
| `gridcraft-cli-<Version>-macos-universal.zip` | das Kommandozeilenprogramm (nur bei Bedarf) |

Nur für diesen Mac und schneller: `packaging/macos/package.sh --arch aarch64` (Apple Silicon) oder
`--arch x86_64` (Intel); die DMG läuft dann nur auf Macs dieser Sorte.

## 4. Installieren

```sh
open dist/release/gridcraft-*-macos-universal.dmg
```

Im Fenster `GridCraft` auf `Applications` ziehen, dann die DMG auswerfen. Läuft GridCraft noch,
vorher beenden, sonst wird die alte Fassung nicht ersetzt.

**Signatur:** Ohne Apple-Entwicklerzertifikat signiert das Skript nur „ad hoc“ (die Warnung
`ad-hoc signing … not notarized` beim Bau ist normal). Auf dem Mac, der gebaut hat, startet
GridCraft ganz normal. Auf einem anderen Mac meldet macOS beim ersten Start, der Entwickler sei nicht
verifiziert: dann einmal mit Rechtsklick › Öffnen › Öffnen starten, oder im Terminal
`xattr -dr com.apple.quarantine /Applications/GridCraft.app`.

## 5. Später aktualisieren

Schritt 2 (`git pull`), Schritt 3 (`packaging/macos/package.sh`), Schritt 4 (installieren).

## Wenn etwas hakt

| Meldung | Abhilfe |
|---|---|
| `xcrun: error: invalid active developer path` | `xcode-select --install` |
| `target may not be installed` oder `can't find crate for core` | `rustup target add aarch64-apple-darwin x86_64-apple-darwin` |
| `package … requires rustc 1.90` | `rustup update` |
| `the lock file … needs to be updated but --locked was passed` | Der Stand ist nicht sauber geholt: `git status`, `git restore .`, `git pull` |
| `hdiutil: … Resource busy` | Eine alte GridCraft-DMG ist noch eingebunden: im Finder auswerfen, noch einmal bauen |
| Kein Platz | Der Bau braucht einige GB in `target/`; `cargo clean` räumt alte Bauten weg |

Nur ausprobieren, ohne DMG: `cargo run --release -p gridcraft`.
