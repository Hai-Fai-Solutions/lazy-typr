# WebRTC VAD Integration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a two-stage VAD pipeline where WebRTC VAD acts as a noise gate on top of the existing RMS VAD, reducing false-positive transcriptions from background noise.

**Architecture:** The existing `Vad` struct in `src/audio/vad.rs` remains unchanged as stage 1 (RMS energy gate). A new `WebrtcVadFilter` in `src/audio/webrtc_vad.rs` forms stage 2; it classifies fixed 10ms frames and suppresses non-speech chunks during segment accumulation. `SpeechEnd` is always forwarded regardless of the last WebRTC frame (to avoid clipping).

**Tech Stack:** `webrtc-vad` crate (Rust bindings for Google's WebRTC VAD C library, BSD-3-clause, statically linked). Requires `cmake` and a C++ compiler at build time — already satisfied by `whisper-rs`.

**Design doc:** `docs/plans/2026-03-08-webrtc-vad-design.md`

---

### Task 1: Add `webrtc-vad` dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add the dependency**

In `Cargo.toml`, under `[dependencies]`, add after the `cpal` line:

```toml
webrtc-vad = "0.4"
```

**Step 2: Verify it compiles**

```bash
cargo build 2>&1 | tail -5
```

Expected: build succeeds (may take a moment to compile the C library). If `cmake` or a C++ compiler is missing, install them (`cmake clang` on Arch, `cmake clang` on Debian/Ubuntu). `whisper-rs` already requires these, so they should already be present.

**Step 3: Check the license**

```bash
cargo deny check licenses 2>&1 || echo "cargo-deny not installed — skip"
```

If `cargo-deny` is available and flags anything unexpected, investigate before continuing.

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add webrtc-vad dependency"
```

---

### Task 2: Create `WebrtcVadFilter` (TDD)

**Files:**
- Create: `src/audio/webrtc_vad.rs`
- Modify: `src/audio/mod.rs` (add `mod webrtc_vad;`)

**Step 1: Write the failing tests**

Create `src/audio/webrtc_vad.rs` with only the tests (no implementation yet):

```rust
use webrtc_vad::{Vad, VadMode, SampleRate};

const FRAME_SAMPLES: usize = 160; // 10ms @ 16kHz

pub struct WebrtcVadFilter {
    vad: Vad,
    frame_buf: Vec<i16>,
}

impl WebrtcVadFilter {
    pub fn new(aggressiveness: u8) -> Self {
        let mode = match aggressiveness {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            _ => VadMode::VeryAggressive,
        };
        Self {
            vad: Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, mode),
            frame_buf: Vec::with_capacity(FRAME_SAMPLES),
        }
    }

    /// Feed f32 samples (16kHz mono). Returns true if any complete 10ms frame
    /// was classified as speech.
    pub fn is_speech(&mut self, samples: &[f32]) -> bool {
        todo!()
    }
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * 32767.0) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_not_speech() {
        let mut filter = WebrtcVadFilter::new(2);
        // 1 second of silence — should never classify as speech
        let silence = vec![0.0f32; 16000];
        assert!(!filter.is_speech(&silence));
    }

    #[test]
    fn partial_frame_returns_false() {
        let mut filter = WebrtcVadFilter::new(2);
        // Feed fewer than 160 samples — no complete frame, must return false
        let partial = vec![0.5f32; 100];
        assert!(!filter.is_speech(&partial));
    }

    #[test]
    fn partial_frame_is_buffered_and_completes_next_call() {
        let mut filter = WebrtcVadFilter::new(2);
        // Feed 100 samples, then 60 more — together they form exactly one frame
        let part_a = vec![0.0f32; 100];
        let part_b = vec![0.0f32; 60];
        assert!(!filter.is_speech(&part_a)); // no complete frame yet
        // Second call completes the frame; silence should still return false
        let _ = filter.is_speech(&part_b); // result doesn't matter, just must not panic
    }

    #[test]
    fn f32_to_i16_zero() {
        assert_eq!(f32_to_i16(0.0), 0);
    }

    #[test]
    fn f32_to_i16_positive_clamps() {
        assert_eq!(f32_to_i16(1.0), 32767);
        assert_eq!(f32_to_i16(2.0), 32767); // clamped
    }

    #[test]
    fn f32_to_i16_negative_clamps() {
        assert_eq!(f32_to_i16(-1.0), -32767);
        assert_eq!(f32_to_i16(-2.0), -32767); // clamped
    }

    #[test]
    fn new_accepts_all_aggressiveness_levels() {
        // Must not panic for any valid level
        for level in 0u8..=3 {
            let _ = WebrtcVadFilter::new(level);
        }
    }
}
```

**Step 2: Register the module**

In `src/audio/mod.rs`, add at the top with the other `mod` declarations:

```rust
mod webrtc_vad;
pub use webrtc_vad::WebrtcVadFilter;
```

**Step 3: Run tests to confirm they fail**

```bash
cargo test webrtc_vad 2>&1 | tail -20
```

Expected: compile error on `todo!()` panic or test failures — confirms tests are wired up correctly.

**Step 4: Implement `is_speech`**

Replace `todo!()` in `src/audio/webrtc_vad.rs` with:

```rust
pub fn is_speech(&mut self, samples: &[f32]) -> bool {
    let i16_samples: Vec<i16> = samples.iter().map(|&s| f32_to_i16(s)).collect();
    self.frame_buf.extend_from_slice(&i16_samples);

    let mut any_speech = false;
    while self.frame_buf.len() >= FRAME_SAMPLES {
        let frame: Vec<i16> = self.frame_buf.drain(..FRAME_SAMPLES).collect();
        if self.vad.is_voice_segment(&frame).unwrap_or(false) {
            any_speech = true;
        }
    }
    any_speech
}
```

**Step 5: Run tests to confirm they pass**

```bash
cargo test webrtc_vad 2>&1 | tail -20
```

Expected: all tests pass.

**Step 6: Commit**

```bash
git add src/audio/webrtc_vad.rs src/audio/mod.rs
git commit -m "feat(audio): add WebrtcVadFilter wrapping webrtc-vad crate"
```

---

### Task 3: Add `webrtc_vad_aggressiveness` to Config (TDD)

**Files:**
- Modify: `src/config.rs`

**Step 1: Write failing config tests**

At the bottom of the `#[cfg(test)]` block in `src/config.rs`, add:

```rust
#[test]
fn test_default_webrtc_vad_aggressiveness_is_2() {
    assert_eq!(Config::default().webrtc_vad_aggressiveness, 2);
}

#[test]
fn test_webrtc_vad_aggressiveness_round_trips_through_json() {
    let cfg = Config {
        webrtc_vad_aggressiveness: 3,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.webrtc_vad_aggressiveness, 3);
}

#[test]
fn test_webrtc_vad_aggressiveness_absent_in_legacy_json_defaults_to_2() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.webrtc_vad_aggressiveness, 2);
}
```

**Step 2: Run to confirm failure**

```bash
cargo test test_default_webrtc_vad_aggressiveness 2>&1 | tail -10
```

Expected: compile error — field doesn't exist yet.

**Step 3: Add the field to `Config`**

In `src/config.rs`, add a helper function before `impl Default for Config`:

```rust
fn default_webrtc_vad_aggressiveness() -> u8 {
    2
}
```

Add the field to the `Config` struct (after `gpu_device`):

```rust
/// WebRTC VAD aggressiveness level (0 = least aggressive, 3 = most aggressive).
/// Higher values filter more background noise but may clip quiet speech.
#[serde(default = "default_webrtc_vad_aggressiveness")]
pub webrtc_vad_aggressiveness: u8,
```

Add the default value to `Config::default()`:

```rust
webrtc_vad_aggressiveness: default_webrtc_vad_aggressiveness(),
```

**Step 4: Run tests to confirm they pass**

```bash
cargo test config 2>&1 | tail -20
```

Expected: all config tests pass, including the three new ones.

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add webrtc_vad_aggressiveness field (default: 2)"
```

---

### Task 4: Add `--webrtc-vad-aggressiveness` CLI flag

**Files:**
- Modify: `src/main.rs`

**Step 1: Add the CLI argument**

In `src/main.rs`, add to the `Args` struct after the `gpu_device` field:

```rust
/// WebRTC VAD aggressiveness level 0-3 (higher = more noise rejection)
#[arg(long, value_name = "0-3")]
webrtc_vad_aggressiveness: Option<u8>,
```

**Step 2: Wire it into the config merge block**

In the `fn main()` config merge section (after `if let Some(key) = args.ptt_key`), add:

```rust
if let Some(level) = args.webrtc_vad_aggressiveness {
    if level > 3 {
        eprintln!("Error: webrtc_vad_aggressiveness must be 0-3, got {}", level);
        std::process::exit(1);
    }
    config.webrtc_vad_aggressiveness = level;
}
```

**Step 3: Add a startup log line**

In the logging block after `info!("GPU inference: ...")`, add:

```rust
info!(
    "WebRTC VAD aggressiveness: {} ({})",
    config.webrtc_vad_aggressiveness,
    match config.webrtc_vad_aggressiveness {
        0 => "quality",
        1 => "low-bitrate",
        2 => "aggressive",
        _ => "very-aggressive",
    }
);
```

**Step 4: Build to verify**

```bash
cargo build 2>&1 | tail -10
```

Expected: compiles cleanly.

**Step 5: Smoke-test the flag**

```bash
./target/debug/whisper-type --help | grep webrtc
```

Expected: `--webrtc-vad-aggressiveness <0-3>` appears in help output.

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): add --webrtc-vad-aggressiveness flag (0-3)"
```

---

### Task 5: Wire `WebrtcVadFilter` into the audio pipeline (TDD)

**Files:**
- Modify: `src/audio/mod.rs`
- Modify: `tests/vad_pipeline.rs`

**Step 1: Write failing integration test**

At the bottom of `tests/vad_pipeline.rs`, add:

```rust
// ── WebRTC VAD two-stage gate ─────────────────────────────────────────────

/// Verify that the two-stage pipeline (RMS + WebRTC) does not send a segment
/// when the audio is pure silence — WebRTC VAD should reject it even if RMS
/// would have let it through (e.g. with a very low threshold).
///
/// We test this at the unit level by verifying WebrtcVadFilter rejects silence,
/// since wiring into AudioCapture requires actual audio hardware.
#[test]
fn webrtc_vad_rejects_silence() {
    use whisper_type::audio::WebrtcVadFilter;
    let mut filter = WebrtcVadFilter::new(2);
    let silence = vec![0.0f32; 16000]; // 1 second
    assert!(
        !filter.is_speech(&silence),
        "WebRTC VAD must not classify silence as speech"
    );
}

#[test]
fn webrtc_vad_accepts_valid_aggressiveness_range() {
    use whisper_type::audio::WebrtcVadFilter;
    for level in 0u8..=3 {
        let mut filter = WebrtcVadFilter::new(level);
        let silence = vec![0.0f32; 160];
        let _ = filter.is_speech(&silence); // must not panic
    }
}
```

**Step 2: Run to confirm failure**

```bash
cargo test webrtc_vad_rejects 2>&1 | tail -10
```

Expected: compile error — `WebrtcVadFilter` not yet public at crate level.

**Step 3: Expose `WebrtcVadFilter` from the crate's public API**

In `src/lib.rs`, find the `pub mod audio` line. Verify `WebrtcVadFilter` is re-exported. The `mod.rs` step in Task 2 added `pub use webrtc_vad::WebrtcVadFilter;` — check it's visible through `whisper_type::audio::WebrtcVadFilter`. If `src/lib.rs` doesn't already have `pub mod audio`, add it. Check current contents:

```bash
cat src/lib.rs
```

Ensure it contains:
```rust
pub mod audio;
```

**Step 4: Wire `WebrtcVadFilter` into `handle_audio_vad`**

In `src/audio/mod.rs`, update `handle_audio_vad` signature to accept the filter:

```rust
fn handle_audio_vad(
    resampled: &[f32],
    segment: &Arc<Mutex<Vec<f32>>>,
    vad: &Arc<Mutex<Vad>>,
    webrtc_filter: Option<&mut WebrtcVadFilter>,
    tx: &Sender<Vec<f32>>,
    max_samples: usize,
) {
```

Update the `SpeechStart | VadEvent::None` arm to gate on WebRTC VAD:

```rust
VadEvent::SpeechStart | VadEvent::None => {
    if vad.lock().unwrap().is_speaking {
        let passes_webrtc = webrtc_filter
            .map(|f| f.is_speech(resampled))
            .unwrap_or(true); // no filter = pass through
        if passes_webrtc {
            seg.extend_from_slice(resampled);
            if seg.len() > max_samples {
                let drain_to = seg.len() - max_samples;
                seg.drain(..drain_to);
            }
        }
    }
}
```

The `SpeechEnd` arm stays unchanged — always forwards the accumulated segment.

**Step 5: Update `dispatch` to thread the filter through**

Update `dispatch` in `src/audio/mod.rs`:

```rust
fn dispatch(
    resampled: &[f32],
    segment: &Arc<Mutex<Vec<f32>>>,
    vad: &Arc<Mutex<Vad>>,
    webrtc_filter: Option<&mut WebrtcVadFilter>,
    ptt_active: Option<&Arc<AtomicBool>>,
    ptt_was_active: &mut bool,
    tx: &Sender<Vec<f32>>,
    max_samples: usize,
) {
    if let Some(ptt) = ptt_active {
        handle_audio_ptt(resampled, segment, ptt, ptt_was_active, tx, max_samples);
    } else {
        handle_audio_vad(resampled, segment, vad, webrtc_filter, tx, max_samples);
    }
}
```

**Step 6: Construct `WebrtcVadFilter` in `AudioCapture::run` and pass it**

In `AudioCapture::run`, after the `Vad` construction block, add:

```rust
let mut webrtc_filter = WebrtcVadFilter::new(vad_cfg.webrtc_vad_aggressiveness);
```

Then update both `build_input_stream` closures (F32 and I16 branches). Each closure currently captures `vad_clone` etc. Both closures need to capture a `mut webrtc_filter`. Since closures can't share `&mut`, move the filter into the closure directly.

For the F32 branch:
```rust
let mut wrtc = WebrtcVadFilter::new(vad_cfg.webrtc_vad_aggressiveness);
self.device.build_input_stream(
    &stream_config,
    move |data: &[f32], _| {
        let resampled = prepare_samples(data, channels, needs_resample, resample_ratio);
        dispatch(
            &resampled,
            &segment_w,
            &vad_clone,
            Some(&mut wrtc),
            ptt.as_ref(),
            &mut ptt_was_active,
            &audio_tx_inner,
            max_samples,
        );
    },
    err_fn,
    None,
)?
```

For the I16 branch, do the same (create `let mut wrtc2 = WebrtcVadFilter::new(...)` before the closure).

**Step 7: Run all tests**

```bash
cargo test 2>&1 | tail -30
```

Expected: all tests pass. Pay attention to any new compile errors from the closure capture changes.

**Step 8: Full test suite with output**

```bash
cargo test -- --nocapture 2>&1 | grep -E "(test .* ok|FAILED|error)"
```

Expected: all `ok`, no `FAILED`.

**Step 9: Commit**

```bash
git add src/audio/mod.rs src/audio/webrtc_vad.rs tests/vad_pipeline.rs
git commit -m "feat(audio): wire WebrtcVadFilter as stage-2 noise gate in VAD pipeline"
```

---

### Task 6: Final validation

**Step 1: Clean build**

```bash
cargo build --release 2>&1 | tail -10
```

Expected: release build succeeds.

**Step 2: Clippy**

```bash
cargo clippy -- -D warnings 2>&1 | tail -20
```

Fix any warnings before proceeding.

**Step 3: Format**

```bash
cargo fmt
git diff --name-only
```

If any files were reformatted, stage and amend or create a new commit:

```bash
git add -u
git commit -m "style: cargo fmt after webrtc-vad integration"
```

**Step 4: Dry-run smoke test**

```bash
./target/release/whisper-type --dry-run --webrtc-vad-aggressiveness 3 --help
```

Expected: help text shows the new flag.

**Step 5: Final commit if needed**

```bash
git log --oneline -6
```

Verify commits are clean and in logical order.
