# whisper-type 🎤

Echtzeit Speech-to-Text für Linux — transkribiert deine Sprache lokal mit OpenAI Whisper und tippt den Text direkt in das aktive Eingabefeld.

**Keine Cloud. Keine API-Keys. Vollständig offline.**

---

## Features

- 🎙️ **Echtzeit-Aufnahme** via CPAL (ALSA/PulseAudio/Pipewire)
- 🧠 **Lokale KI** via Whisper (ggml, kein Internet nötig)
- ⌨️ **Automatisches Tippen** in jedes fokussierte Textfeld (Wayland: `wtype`, X11: `xdotool`)
- 🔇 **Voice Activity Detection** — sendet nur wenn du wirklich sprichst
- 🎯 **Push-to-Talk** — optional: Taste halten zum Aufnehmen (VAD wird umgangen)
- 🌍 **Mehrsprachig** — Deutsch, Englisch, und alle anderen Whisper-Sprachen
- ⚡ **Multi-threaded** — Audio, VAD, Whisper und Typer laufen parallel
- 🖥️ **Wayland & X11** — erkennt automatisch die Display-Umgebung

---

## Schnellstart

```bash
# 1. Setup (einmalig)
chmod +x setup.sh
./setup.sh

# 2. Starten
whisper-type

# 3. Sprechen — Text erscheint im aktiven Fenster
```

---

## Installation (manuell)

### System-Abhängigkeiten

**Arch Linux:**
```bash
sudo pacman -S xdotool alsa-lib pkgconf base-devel xclip
# Wayland (Hyprland, Sway, etc.):
sudo pacman -S wtype wl-clipboard
```

**Debian/Ubuntu:**
```bash
sudo apt install xdotool libasound2-dev pkg-config build-essential xclip
# Wayland:
sudo apt install wtype wl-clipboard
```

### Whisper Model herunterladen

```bash
mkdir -p ~/.local/share/whisper-type
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \
     -O ~/.local/share/whisper-type/ggml-base.bin
```

| Modell | Größe | Qualität | RAM |
|--------|-------|----------|-----|
| tiny   | 75 MB | ⭐⭐ | ~1 GB |
| **base** | **142 MB** | **⭐⭐⭐** | **~1 GB** |
| small  | 466 MB | ⭐⭐⭐⭐ | ~2 GB |
| medium | 1.5 GB | ⭐⭐⭐⭐⭐ | ~5 GB |

### Build

```bash
cargo build --release
cp target/release/whisper-type ~/.local/bin/
```

---

## Verwendung

```
USAGE:
    whisper-type [OPTIONS]

OPTIONS:
    -m, --model <PATH>        Pfad zur GGML-Modelldatei
    -d, --device <NAME>       Audio-Eingabegerät (Standard: System-Default)
    -l, --language <LANG>     Sprache (de, en, fr, ...) [Standard: de]
        --silence-ms <MS>     Stille-Schwelle in ms [Standard: 800]
        --list-devices        Verfügbare Audio-Geräte anzeigen
        --dry-run             Text auf stdout ausgeben statt zu tippen
        --ptt-key <KEY>       Push-to-Talk Taste (z.B. KEY_SPACE, KEY_CAPSLOCK, KEY_F1)
        --log-level <LEVEL>   Log-Verbosität (error, warn, info, debug, trace)
    -h, --help                Hilfe anzeigen
```

### Beispiele

```bash
# Deutsch (Standard)
whisper-type

# Englisch
whisper-type --language en

# Anderes Mikrofon
whisper-type --list-devices
whisper-type --device "USB Audio"

# Schnellere Reaktion (500ms Pause reicht)
whisper-type --silence-ms 500

# Testen ohne zu tippen
whisper-type --dry-run

# Größeres Modell für bessere Genauigkeit
whisper-type --model ~/.local/share/whisper-type/ggml-small.bin

# Detaillierte Logs für Fehlersuche
whisper-type --log-level debug

# Nur Fehler anzeigen
whisper-type --log-level warn

# Push-to-Talk: Leertaste halten zum Aufnehmen
whisper-type --ptt-key KEY_SPACE

# Push-to-Talk: Capslock halten (gut für längere Aufnahmen)
whisper-type --ptt-key KEY_CAPSLOCK

# Push-to-Talk: F12 als dedizierte PTT-Taste
whisper-type --ptt-key KEY_F12
```

---

## Konfiguration

Gespeichert unter `~/.config/whisper-type/config.json`:

```json
{
  "model_path": "/home/user/.local/share/whisper-type/ggml-base.bin",
  "device_name": null,
  "language": "de",
  "silence_threshold_ms": 800,
  "min_speech_ms": 300,
  "max_buffer_secs": 30.0,
  "vad_threshold": 0.01,
  "log_level": "info",
  "ptt_key": null
}
```

| Parameter | Beschreibung | Standard |
|-----------|-------------|---------|
| `silence_threshold_ms` | Wie lange Stille, bis Segment gesendet wird (nur VAD-Modus) | `800` |
| `min_speech_ms` | Minimale Sprachzeit, sonst verworfen (nur VAD-Modus) | `300` |
| `vad_threshold` | Energie-Schwelle für Spracherkennung (0.0–1.0) | `0.01` |
| `max_buffer_secs` | Maximale Aufnahmedauer pro Segment | `30.0` |
| `log_level` | Log-Verbosität: `error`, `warn`, `info`, `debug`, `trace` | `"info"` |
| `ptt_key` | Push-to-Talk Taste (z.B. `"KEY_SPACE"`). `null` = VAD-Modus | `null` |

**Log-Level Priorität** (von niedrig nach hoch): `config.json` → `--log-level` Flag → `RUST_LOG` Umgebungsvariable

### Push-to-Talk einrichten

PTT liest direkt vom Kernel (`/dev/input`). Der Benutzer muss in der `input`-Gruppe sein:

```bash
sudo usermod -aG input $USER
# Neu einloggen oder:
newgrp input
```

Unterstützte Tasten: `KEY_SPACE`, `KEY_CAPSLOCK`, `KEY_SCROLLLOCK`, `KEY_PAUSE`,
`KEY_LEFTCTRL`, `KEY_RIGHTCTRL`, `KEY_LEFTSHIFT`, `KEY_RIGHTSHIFT`,
`KEY_LEFTALT`, `KEY_RIGHTALT`, `KEY_LEFTMETA`, `KEY_F1`–`KEY_F12`

> Das `KEY_`-Präfix ist optional: `SPACE` und `KEY_SPACE` sind gleichwertig.

---

## Wie es funktioniert

```
Mikrofon (CPAL)
     │
     ▼
Downmix → Mono
     │
     ▼
Resampling → 16kHz
     │
     ▼
VAD (Energy-based)
     │  speech end detected
     ▼
Whisper (ggml, lokal)
     │
     ▼
Text-Filter (Halluzinationen)
     │
     ▼
wtype (Wayland) / xdotool (X11) → aktives Fenster
```

---

## Fehlerbehebung

**`xdotool not found`**
```bash
sudo apt install xdotool
```

**`No default input device found`**
```bash
# PulseAudio/Pipewire prüfen
pactl list sources short
whisper-type --list-devices
```

**Text wird nicht getippt (Wayland)**
`whisper-type` erkennt Wayland automatisch und nutzt `wtype`. Stelle sicher, dass `wtype` installiert ist:
```bash
# Arch:
sudo pacman -S wtype
# Debian/Ubuntu:
sudo apt install wtype
```

**Whisper-Modell nicht gefunden**  
```bash
# Standardpfad:
ls ~/.local/share/whisper-type/
# Oder explizit angeben:
whisper-type --model /pfad/zum/modell.bin
```

**Zu viele Halluzinationen bei Stille**
```bash
# VAD-Schwelle erhöhen (in ~/.config/whisper-type/config.json):
"vad_threshold": 0.02
# Oder Push-to-Talk verwenden — nimmt nur auf, wenn Taste gehalten wird:
whisper-type --ptt-key KEY_SPACE
```

**PTT: "No input device found"**
Der Benutzer ist nicht in der `input`-Gruppe:
```bash
sudo usermod -aG input $USER
# Neu einloggen, dann erneut versuchen
```

---

## Developer Lifecycle

### Prerequisites

- Rust toolchain (stable): `rustup install stable`
- System dependencies (see [Installation](#installation-manuell) above)
- `whisper-rs` requires a C++ compiler and `cmake` for building whisper.cpp:
  ```bash
  # Arch:
  sudo pacman -S cmake clang
  # Debian/Ubuntu:
  sudo apt install cmake clang
  ```

### Clone & build

```bash
git clone <repo-url>
cd lazy-typr

# Debug build (fast compile, slow inference)
cargo build

# Release build (optimised — use this for actual transcription)
cargo build --release
```

### Run locally

```bash
# Debug binary
cargo run -- --dry-run

# Release binary
./target/release/whisper-type --dry-run
```

### Tests

```bash
# Run all tests
cargo test

# Run a specific test file
cargo test --test vad_pipeline
cargo test --test config_integration
cargo test --test ptt_key_coverage

# Show test output (tracing logs)
cargo test -- --nocapture
```

Integration tests live in [tests/](tests/). They cover VAD pipeline logic, config loading, and PTT key name parsing — no audio hardware or Whisper model required.

### Project layout

```
src/
├── main.rs          # CLI entry point (clap), thread spawn, shutdown
├── lib.rs           # Public re-exports for integration tests
├── config.rs        # Config struct, JSON load/merge with CLI flags
├── audio/
│   ├── mod.rs       # CPAL capture, downmix, resample, VAD/PTT dispatch
│   └── vad.rs       # Energy-based Voice Activity Detection
├── transcriber.rs   # Whisper inference thread
├── typer.rs         # wtype / xdotool dispatch thread
└── ptt.rs           # evdev push-to-talk monitor thread
tests/
├── config_integration.rs
├── vad_pipeline.rs
└── ptt_key_coverage.rs
```

### Branching & releases

| Branch | Purpose |
|--------|---------|
| `main` | Stable, tagged releases |
| `develop` | Integration branch — all PRs target this |
| `feature/*` | Short-lived feature branches off `develop` |

```bash
# Start a feature
git checkout develop
git checkout -b feature/my-feature

# Merge back
git checkout develop
git merge --no-ff feature/my-feature
```

Releases are cut from `develop` → `main` via PR, then tagged:

```bash
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0
```

### Linting & formatting

```bash
cargo fmt                  # Format code
cargo fmt --check          # Check only (CI)
cargo clippy -- -D warnings  # Lint (treat warnings as errors)
```

---

## Lizenz

MIT
