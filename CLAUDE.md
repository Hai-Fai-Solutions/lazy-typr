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
| Text output | `xdotool type` into the focused window; clipboard paste as fallback |

## Architecture

Three threads communicate via crossbeam channels:

1. **Audio thread** (`audio.rs`) — captures microphone via CPAL, downmixes to mono, resamples to 16 kHz
2. **Transcriber thread** (`transcriber.rs`) — receives speech segments, runs Whisper inference via `whisper-rs`
3. **Typer thread** (`typer.rs`) — receives transcribed text, types it into the focused window via `xdotool`

Voice Activity Detection (`vad.rs`) runs inside the audio thread and only forwards a buffer to Whisper when a speech segment ends (energy drops below threshold for `silence_threshold_ms` ms).

## Key dependencies

- `whisper-rs` — Rust bindings for whisper.cpp (ggml)
- `cpal` — cross-platform audio input (ALSA / PulseAudio / Pipewire)
- `arboard` — clipboard access (fallback path)
- `crossbeam-channel` — lock-free channels between threads
- `clap` — CLI argument parsing
- `xdotool` (system) — types text into the active X11 window

## Configuration

Stored at `~/.config/whisper-type/config.json`. Key parameters:

- `model_path` — path to the downloaded `.bin` Whisper model
- `language` — Whisper language code (default: `"de"`)
- `silence_threshold_ms` — pause duration before a segment is sent (default: 800 ms)
- `vad_threshold` — RMS energy threshold for voice detection (default: 0.01)

## Known limitations / future work

- **Wayland**: `xdotool` only works under X11/XWayland. Pure Wayland would require `ydotool` + `uinput`.
- No push-to-talk yet (CLI flag `--ptt-key` is scaffolded but not implemented).
- No system tray icon yet.
- Ollama integration was explicitly excluded but the architecture (a post-processing step between transcriber and typer) would make it straightforward to add later.
