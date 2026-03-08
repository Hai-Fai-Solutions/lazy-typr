# WebRTC VAD Integration — Design Document

**Date:** 2026-03-08
**Status:** Approved
**Branch:** `feature/webrtc-vad`

## Problem

The existing energy-based (RMS) VAD in `src/audio/vad.rs` produces false positives: background
noise (keyboard clicks, fan hum, music, TV) exceeds the energy threshold and triggers unwanted
transcription segments. A single `vad_threshold` value is brittle and requires per-environment
tuning.

## Goal

Reduce false-positive transcriptions caused by background noise without affecting recall for
genuine close-mic speech.

## Approach

Add a **two-stage gate** to the VAD pipeline. The existing RMS `Vad` struct remains stage 1
unchanged. A new `WebrtcVadFilter` wrapper around the `webrtc-vad` crate becomes stage 2. Both
stages must agree before audio is accumulated into a transcription segment.

The `webrtc-vad` crate provides safe Rust bindings to Google's WebRTC VAD C library (BSD 3-clause
license, statically linked — no runtime `.so` dependency).

## Pipeline

```
audio chunk (16kHz mono f32)
        │
        ▼
  [Stage 1: RMS VAD]           src/audio/vad.rs — unchanged
  energy > threshold?
        │ yes
        ▼
  [Stage 2: WebRTC VAD]        src/audio/webrtc_vad.rs — new
  is this a voiced frame?
        │ yes
        ▼
  accumulate in segment buffer
        │
        ▼  (on silence timeout from Stage 1)
  emit SpeechEnd → transcriber
```

Stage 2 operates on fixed **10ms frames** (160 samples @ 16kHz) as required by the WebRTC VAD
algorithm. The wrapper buffers partial frames internally. When Stage 1 declares `SpeechEnd`, the
accumulated segment is forwarded regardless of the last WebRTC frame result, to avoid cutting off
trailing voiced audio. WebRTC VAD acts as a **noise gate during accumulation** only — chunks it
rejects are simply not appended to the segment buffer.

## Code Structure

### New file: `src/audio/webrtc_vad.rs`

```rust
pub struct WebrtcVadFilter {
    vad: webrtc_vad::Vad,
    aggressiveness: u8,   // 0–3, stored for logging/debug
    frame_buf: Vec<i16>,  // accumulates samples until a full 10ms frame (160 samples)
}

impl WebrtcVadFilter {
    pub fn new(aggressiveness: u8) -> Self { ... }
    /// Feed f32 samples; returns true if any complete 10ms frame was voiced.
    pub fn is_speech(&mut self, samples: &[f32]) -> bool { ... }
}
```

Internals:
- Convert `f32 → i16` (multiply by 32768, clamp)
- Buffer samples until 160-sample frames are complete
- Run `webrtc_vad::Vad::is_voice_segment` on each complete frame
- Return `true` if any frame in the chunk was classified as speech

### Modified: `src/audio/mod.rs`

- `AudioCapture::run` constructs a `WebrtcVadFilter` alongside the existing `Vad`
- `handle_audio_vad` gains an `Option<&mut WebrtcVadFilter>` parameter
- When `Some`, samples are only appended to the segment buffer if `webrtc_vad.is_speech(samples)` returns `true`
- `SpeechEnd` is emitted unconditionally when Stage 1 silence timeout fires (avoids clipping)

### Modified: `src/config.rs`

```rust
pub webrtc_vad_aggressiveness: u8,  // default: 2
```

### Modified: CLI (`src/main.rs`)

```
--webrtc-vad-aggressiveness <0-3>
```

Override priority (lowest → highest): `config.json` → CLI flag.

## Configuration

| Parameter | Type | Default | Range | Location |
|-----------|------|---------|-------|----------|
| `webrtc_vad_aggressiveness` | `u8` | `2` | `0–3` | `config.json`, `--webrtc-vad-aggressiveness` |

Aggressiveness levels:
- `0` — least aggressive, accepts most audio, minimal noise rejection
- `1` — low aggressiveness
- `2` — moderate (recommended default, filters most noise without clipping speech)
- `3` — most aggressive, rejects anything not clearly voiced

## Error Handling

- **Invalid aggressiveness value (> 3):** startup error, process exits with:
  `"webrtc_vad_aggressiveness must be 0–3, got N"`
- **`webrtc-vad` init failure:** fatal error, process exits — silent fallback to RMS-only would
  hide misconfiguration

## Licensing

- Google WebRTC VAD C library: **BSD 3-clause** (permissive, attribution only)
- `webrtc-vad` Rust crate: **MIT / BSD** (permissive)

No copyleft (GPL/LGPL/AGPL) in the dependency chain. Verify with `cargo deny check licenses`
after adding the dependency.

## Testing

| Test | Location | What it covers |
|------|----------|----------------|
| Sine wave classified as speech | `src/audio/webrtc_vad.rs` | Basic voiced detection |
| Silence classified as non-speech | `src/audio/webrtc_vad.rs` | Basic noise rejection |
| f32→i16 conversion correctness | `src/audio/webrtc_vad.rs` | No clipping, correct scale |
| Partial frame buffering | `src/audio/webrtc_vad.rs` | Frames smaller than 160 samples don't emit |
| Two-layer pipeline emits SpeechEnd | `tests/vad_pipeline.rs` | End-to-end: only voiced audio triggers segment |

Existing VAD tests in `src/audio/vad.rs` remain untouched.

## Dependencies

Add to `Cargo.toml`:
```toml
webrtc-vad = "0.4"
```

Requires `cmake` and a C++ compiler at build time (already required by `whisper-rs`).
