# CLAUDE.md — Project Context

## What this project is

`whisper-type` is a Linux desktop app written in Rust that performs **real-time, offline speech-to-text** and automatically types the transcribed text into whatever input field is currently focused on screen.

## Decisions made in our conversation

| Topic | Decision |
|-------|----------|
| Language | Rust |
| Platform | Linux only |
| Speech recognition | Local via Whisper (ggml, fully offline) |
| Ollama / KI-Modell | Not used — transcription only, no LLM post-processing |
| Text output | `wtype` on Wayland, `xdotool type` on X11 — auto-detected at runtime; clipboard paste as fallback |
| Logging | `tracing` + `tracing-subscriber`; level configurable via `--log-level`, `config.json`, or `RUST_LOG` |
| Push-to-talk | `evdev` reads `/dev/input` directly — works on X11 and Wayland; user must be in `input` group |

## Architecture

Three threads communicate via crossbeam channels:

1. **Audio thread** (`audio/mod.rs`) — captures microphone via CPAL, downmixes to mono, resamples to 16 kHz; runs either VAD or PTT mode
2. **Transcriber thread** (`transcriber.rs`) — receives speech segments, runs Whisper inference via `whisper-rs`
3. **Typer thread** (`typer.rs`) — receives transcribed text; auto-detects Wayland (`wtype`) or X11 (`xdotool`) at startup

**VAD mode** (default): `audio/vad.rs` runs inside the audio thread and forwards a buffer when speech ends (energy drops below threshold for `silence_threshold_ms` ms).

**PTT mode** (`--ptt-key`): `ptt.rs` spawns a monitor thread per input device via `evdev`; sets an `Arc<AtomicBool>` flag. The audio thread accumulates samples while the flag is true and flushes on key release, bypassing VAD entirely.

## Key dependencies

- `whisper-rs` — Rust bindings for whisper.cpp (ggml)
- `cpal` — cross-platform audio input (ALSA / PulseAudio / Pipewire)
- `arboard` — clipboard access (fallback path, supports X11 and Wayland)
- `crossbeam-channel` — lock-free channels between threads
- `clap` — CLI argument parsing
- `tracing` + `tracing-subscriber` — structured logging
- `evdev` — direct kernel input device access for PTT key monitoring (X11 + Wayland)
- `xdotool` (system) — types text into the active X11 window
- `wtype` (system) — types text into the active Wayland window

## Configuration

Stored at `~/.config/whisper-type/config.json`. Key parameters:

- `model_path` — path to the downloaded `.bin` Whisper model
- `language` — Whisper language code (default: `"de"`)
- `silence_threshold_ms` — pause duration before a segment is sent (default: 800 ms)
- `vad_threshold` — RMS energy threshold for voice detection (default: 0.01)
- `log_level` — log verbosity: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"` (default: `"info"`)
- `ptt_key` — PTT key name (e.g. `"KEY_SPACE"`, `"KEY_CAPSLOCK"`); `null` = VAD mode (default)
- `gpu_backend` — GPU backend: `"auto"` (default), `"cuda"`, `"vulkan"`, `"cpu"`. Auto detects NVIDIA → CUDA, else Vulkan, else CPU.
- `gpu_device` — Device index for the active GPU backend (default: `0`)

Override priority (lowest → highest): `config.json` → CLI flag → `RUST_LOG` env var (log level only)

PTT requires the user to be in the `input` group: `sudo usermod -aG input $USER`

## Developer lifecycle

### Build

```bash
cargo build           # debug
cargo build --release # release (LTO, opt-level 3)
```

`whisper-rs` requires cmake, a C++ compiler, and Vulkan headers (`cmake clang vulkan-headers` on Arch; `cmake clang libvulkan-dev` on Debian/Ubuntu).

### Run

```bash
cargo run -- --dry-run                    # no hardware needed
./target/release/whisper-type --dry-run
```

### Test

```bash
cargo test                          # all tests
cargo test --test vad_pipeline      # specific integration test
cargo test -- --nocapture           # show tracing output
```

Integration tests (`tests/`) cover VAD, config loading, and PTT key parsing — no audio hardware or Whisper model required.

### Lint & format

```bash
cargo fmt
cargo clippy -- -D warnings
```

### Branching

- `main` — stable, tagged releases; all PRs target here
- `feature/**` or `bugfix/**` — short-lived branches off `main`

Releases: PR `feature/**` or `bugfix/**` → `main`; cocogitto auto-bumps version and tag on merge.

## Known limitations / future work

- No system tray icon yet.
- Ollama integration was explicitly excluded but the architecture (a post-processing step between transcriber and typer) would make it straightforward to add later.
