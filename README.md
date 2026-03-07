# whisper-type 🎤

Real-time speech-to-text for Linux — transcribes your speech locally with OpenAI Whisper and types the text directly into the active input field.

**No cloud. No API keys. Fully offline.**

---
 

## Features

- 🎙️ **Real-time recording** via CPAL (ALSA/PulseAudio/Pipewire)
- 🧠 **Local AI** via Whisper (ggml, no internet required)
- ⌨️ **Automatic typing** into any focused text field (`ydotool` for KDE/any compositor, `wtype` for wlroots Wayland, `xdotool` for X11)
- 🔇 **Voice Activity Detection** — only sends audio when you are actually speaking
- 🎯 **Push-to-Talk** — optional: hold a key to record (bypasses VAD)
- 🌍 **Multilingual** — German, English, and all other Whisper languages
- ⚡ **Multi-threaded** — audio, VAD, Whisper, and typer run in parallel
- 🚀 **GPU acceleration** — optional NVIDIA (CUDA), AMD (ROCm/hipBLAS), or cross-vendor (Vulkan)
- 🖥️ **Wayland & X11** — automatically detects the display environment

---

## Quick Start

```bash
# 1. Setup (once)
chmod +x setup.sh
./setup.sh

# 2. Start
whisper-type

# 3. Speak — text appears in the active window
```

---

## Installation (manual)

### System Dependencies

**Arch Linux:**
```bash
sudo pacman -S xdotool alsa-lib pkgconf base-devel xclip
# Wayland (Hyprland, Sway, etc.):
sudo pacman -S wtype wl-clipboard
# KDE Wayland (or any compositor — ydotool works everywhere):
sudo pacman -S ydotool
systemctl --user enable --now ydotoold
```

**Debian/Ubuntu:**
```bash
sudo apt install xdotool libasound2-dev pkg-config build-essential xclip
# Wayland:
sudo apt install wtype wl-clipboard
# KDE Wayland (or any compositor):
sudo apt install ydotool
systemctl --user enable --now ydotoold
```

### Download Whisper Model

```bash
mkdir -p ~/.local/share/whisper-type
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \
     -O ~/.local/share/whisper-type/ggml-base.bin
```

| Model | Size | Quality | RAM |
|-------|------|---------|-----|
| tiny   | 75 MB | ⭐⭐ | ~1 GB |
| **base** | **142 MB** | **⭐⭐⭐** | **~1 GB** |
| small  | 466 MB | ⭐⭐⭐⭐ | ~2 GB |
| medium | 1.5 GB | ⭐⭐⭐⭐⭐ | ~5 GB |

### Build

```bash
cargo build --release
cp target/release/whisper-type ~/.local/bin/
```

#### GPU-accelerated builds

The binary must be compiled with the appropriate feature flag for your GPU:

| GPU | Feature flag | Build command |
|-----|---|---|
| NVIDIA | `cuda` | `cargo build --release --features cuda` |
| AMD (ROCm) | `hipblas` | `cargo build --release --features hipblas` |
| NVIDIA or AMD (Vulkan) | `vulkan` | `cargo build --release --features vulkan` |

Build-time requirements:

- **NVIDIA/CUDA** — CUDA Toolkit >= 12 (`nvcc` on `$PATH`)
  - Arch: `sudo pacman -S cuda`
  - Debian/Ubuntu: `sudo apt install nvidia-cuda-toolkit`
- **AMD/ROCm (hipblas)** — ROCm >= 5.x (`hipcc` on `$PATH`)
- **Vulkan** — Vulkan ICD + headers (`vulkan-headers` is required for compilation)
  - Arch (AMD): `sudo pacman -S vulkan-headers vulkan-icd-loader vulkan-radeon`
  - Arch (NVIDIA): `sudo pacman -S vulkan-headers vulkan-nvidia`
  - Debian/Ubuntu: `sudo apt install libvulkan-dev mesa-vulkan-drivers`

The resulting binary only needs the GPU **runtime** (driver + shared libs) on end-user machines — no SDK required.

---

## Usage

```
USAGE:
    whisper-type [OPTIONS]

OPTIONS:
    -m, --model <PATH>        Path to the GGML model file
    -d, --device <NAME>       Audio input device (default: system default)
    -l, --language <LANG>     Language (de, en, fr, ...) [default: de]
        --silence-ms <MS>     Silence threshold in ms [default: 800]
        --list-devices        List available audio devices
        --dry-run             Print text to stdout instead of typing
        --ptt-key <KEY>       Push-to-Talk key (e.g. KEY_SPACE, KEY_CAPSLOCK, KEY_F1)
        --log-level <LEVEL>   Log verbosity (error, warn, info, debug, trace)
        --gpu                 Enable GPU acceleration (binary must include a GPU feature)
        --no-gpu              Disable GPU acceleration (overrides config file)
        --gpu-device <INDEX>  GPU device index [default: 0]
    -h, --help                Show help
```

### Examples

```bash
# German (default)
whisper-type

# English
whisper-type --language en

# Different microphone
whisper-type --list-devices
whisper-type --device "USB Audio"

# Faster response (500ms pause is enough)
whisper-type --silence-ms 500

# Test without typing
whisper-type --dry-run

# Larger model for better accuracy
whisper-type --model ~/.local/share/whisper-type/ggml-small.bin

# Detailed logs for debugging
whisper-type --log-level debug

# Show errors only
whisper-type --log-level warn

# Force CPU mode even when config has use_gpu: true
whisper-type --no-gpu

# Push-to-Talk: hold spacebar to record
whisper-type --ptt-key KEY_SPACE

# Push-to-Talk: hold Capslock (good for longer recordings)
whisper-type --ptt-key KEY_CAPSLOCK

# Push-to-Talk: F12 as a dedicated PTT key
whisper-type --ptt-key KEY_F12
```

---

## Configuration

Stored at `~/.config/whisper-type/config.json`:

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
  "ptt_key": null,
  "use_gpu": false,
  "gpu_device": null
}
```

| Parameter | Description | Default |
|-----------|-------------|---------|
| `silence_threshold_ms` | How long silence must last before a segment is sent (VAD mode only) | `800` |
| `min_speech_ms` | Minimum speech duration; shorter segments are discarded (VAD mode only) | `300` |
| `vad_threshold` | Energy threshold for voice detection (0.0–1.0) | `0.01` |
| `max_buffer_secs` | Maximum recording duration per segment | `30.0` |
| `log_level` | Log verbosity: `error`, `warn`, `info`, `debug`, `trace` | `"info"` |
| `ptt_key` | Push-to-Talk key (e.g. `"KEY_SPACE"`). `null` = VAD mode | `null` |
| `use_gpu` | Enable GPU inference (binary must be built with a GPU feature flag) | `false` |
| `gpu_device` | GPU device index; `null` = let whisper.cpp choose (device 0) | `null` |

**Log level priority** (lowest to highest): `config.json` → `--log-level` flag → `RUST_LOG` environment variable

### Setting Up Push-to-Talk

PTT reads directly from the kernel (`/dev/input`). The user must be in the `input` group:

```bash
sudo usermod -aG input $USER
# Log out and back in, or:
newgrp input
```

Supported keys: `KEY_SPACE`, `KEY_CAPSLOCK`, `KEY_SCROLLLOCK`, `KEY_PAUSE`,
`KEY_LEFTCTRL`, `KEY_RIGHTCTRL`, `KEY_LEFTSHIFT`, `KEY_RIGHTSHIFT`,
`KEY_LEFTALT`, `KEY_RIGHTALT`, `KEY_LEFTMETA`, `KEY_F1`–`KEY_F12`

> The `KEY_` prefix is optional: `SPACE` and `KEY_SPACE` are equivalent.

---

## How It Works

```
Microphone (CPAL)
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
Whisper (ggml, local)
     │
     ▼
Text Filter (hallucinations)
     │
     ▼
wtype (Wayland) / xdotool (X11) → active window
```

---

## Troubleshooting

**`xdotool not found`**
```bash
sudo apt install xdotool
```

**`No default input device found`**
```bash
# Check PulseAudio/Pipewire
pactl list sources short
whisper-type --list-devices
```

**Text is not typed (KDE Wayland)**
`wtype` does not work on KDE Plasma because KDE does not implement the `zwlr_virtual_keyboard_v1`
protocol. Install `ydotool` instead — it writes via `/dev/uinput` and works on any compositor:
```bash
# Arch:
sudo pacman -S ydotool
systemctl --user enable --now ydotoold
# Debian/Ubuntu:
sudo apt install ydotool
systemctl --user enable --now ydotoold
```
`whisper-type` auto-detects `ydotool` on startup and prefers it over `wtype`/`xdotool`.

**`ydotool: Cannot connect to ydotoold`**
The `ydotoold` user daemon is not running:
```bash
systemctl --user enable --now ydotoold
# Verify:
systemctl --user status ydotoold
```

**Text is not typed (wlroots Wayland — Sway, Hyprland, etc.)**
`whisper-type` uses `wtype` on wlroots compositors. Make sure it is installed:
```bash
# Arch:
sudo pacman -S wtype
# Debian/Ubuntu:
sudo apt install wtype
```

**Whisper model not found**
```bash
# Default path:
ls ~/.local/share/whisper-type/
# Or specify explicitly:
whisper-type --model /path/to/model.bin
```

**Too many hallucinations during silence**
```bash
# Increase the VAD threshold (in ~/.config/whisper-type/config.json):
"vad_threshold": 0.02
# Or use Push-to-Talk — only records while the key is held:
whisper-type --ptt-key KEY_SPACE
```

**PTT: "No input device found"**
The user is not in the `input` group:
```bash
sudo usermod -aG input $USER
# Log out and back in, then try again
```

**GPU not used even with `--gpu`**

The binary must be compiled with the matching feature flag — `--gpu` alone is not sufficient if
the binary was built without GPU support:

```bash
# Rebuild with CUDA support (NVIDIA):
cargo build --release --features cuda

# Rebuild with Vulkan support (AMD):
cargo build --release --features vulkan

# Rebuild with ROCm/HIP support (AMD):
cargo build --release --features hipblas
```

Verify it is working:
```bash
whisper-type --gpu --dry-run --log-level debug
# Should print: GPU acceleration: enabled (device 0)
```

**GPU enabled via config but want to test on CPU**

If `config.json` has `"use_gpu": true` (set by setup.sh), the binary uses the GPU even without
`--gpu`. Override it for a single run with `--no-gpu`:

```bash
whisper-type --no-gpu --dry-run
# Should print: GPU acceleration: disabled (CPU only)
```

---

## Developer Lifecycle

### Prerequisites

- Rust toolchain (stable): `rustup install stable`
- System dependencies (see [Installation](#installation-manual) above)
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
| `main` | Stable, tagged releases — all PRs target here |
| `feature/*` | Short-lived feature branches off `main` |
| `bugfix/*` | Short-lived bugfix branches off `main` |

```bash
# Start a feature
git checkout main
git checkout -b feature/my-feature

# Merge back via PR to main
```

Releases are tagged on `main` after merging. Cocogitto auto-bumps the version and tag on merge:

```bash
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0
```

### Linting & formatting

```bash
cargo fmt                    # Format code
cargo fmt --check            # Check only (CI)
cargo clippy -- -D warnings  # Lint (treat warnings as errors)
```

#### Pre-commit hook

A pre-commit hook runs `cargo fmt` automatically before every commit. It is installed by cargo-husky the first time you run:

```bash
cargo test
```

No manual setup needed. After installation, any commit will auto-format staged Rust files and re-stage them before the commit lands.

#### VSCode

Open the repo in VSCode and install the recommended extension (`rust-lang.rust-analyzer`) when prompted. This enables format-on-save, so code is typically already formatted before the pre-commit hook runs.

---

## License

MIT
