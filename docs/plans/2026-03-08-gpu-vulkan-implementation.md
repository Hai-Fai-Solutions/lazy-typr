# GPU Inference via Vulkan — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable optional GPU-accelerated Whisper inference via Vulkan, toggled at runtime through `config.json` and/or the `--gpu` CLI flag.

**Architecture:** Add the `vulkan` feature to `whisper-rs` so whisper.cpp links the Vulkan backend. Expose a `use_gpu: bool` field in `Config` (serde-defaulted to `false`) and a `--gpu` CLI flag. Pass the flag through to `WhisperContextParameters::use_gpu` in `Transcriber::new`. Log effective GPU status on startup.

**Tech Stack:** Rust, whisper-rs 0.11 (vulkan feature), clap 4, serde_json

---

### Task 1: Enable the Vulkan feature in whisper-rs

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add the vulkan feature**

In `Cargo.toml`, find:

```toml
whisper-rs = { version = "0.11", features = [] }
```

Replace with:

```toml
whisper-rs = { version = "0.11", features = ["vulkan"] }
```

**Step 2: Verify it resolves**

Run: `cargo fetch`
Expected: exits 0, no errors.

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: exits 0. (This may take a while — whisper.cpp recompiles with Vulkan support.)

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): enable whisper-rs vulkan feature"
```

---

### Task 2: Add `use_gpu` to Config

**Files:**
- Modify: `src/config.rs`

**Step 1: Write the failing tests first**

In `src/config.rs`, find the `#[cfg(test)]` block and add these tests inside the `tests` module (after `test_default_dry_run_is_false`):

```rust
#[test]
fn test_default_use_gpu_is_false() {
    assert!(!Config::default().use_gpu);
}

#[test]
fn test_use_gpu_round_trips_through_json() {
    let cfg = Config {
        use_gpu: true,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert!(restored.use_gpu);
}

#[test]
fn test_use_gpu_absent_in_legacy_json_defaults_to_false() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert!(!cfg.use_gpu);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_default_use_gpu_is_false test_use_gpu_round_trips_through_json test_use_gpu_absent_in_legacy_json_defaults_to_false -- --nocapture`
Expected: compile error — `use_gpu` field does not exist yet.

**Step 3: Add the field to the struct**

In `src/config.rs`, find:

```rust
    /// PTT key name (e.g. "KEY_SPACE", "KEY_CAPSLOCK"). None = VAD mode.
    #[serde(default)]
    pub ptt_key: Option<String>,
```

Add after it:

```rust
    /// Use GPU (Vulkan) for Whisper inference. Default: false (CPU).
    #[serde(default)]
    pub use_gpu: bool,
```

**Step 4: Add the default value**

In `impl Default for Config`, find the closing brace of the struct literal and add:

```rust
            use_gpu: false,
```

(after `ptt_key: None,`)

**Step 5: Run tests to verify they pass**

Run: `cargo test test_default_use_gpu_is_false test_use_gpu_round_trips_through_json test_use_gpu_absent_in_legacy_json_defaults_to_false -- --nocapture`
Expected: all 3 PASS.

**Step 6: Run full test suite**

Run: `cargo test`
Expected: all tests pass.

**Step 7: Commit**

```bash
git add src/config.rs
git commit -m "feat: add use_gpu field to Config (default: false)"
```

---

### Task 3: Add `--gpu` CLI flag

**Files:**
- Modify: `src/main.rs`

**Step 1: Add the flag to the Args struct**

In `src/main.rs`, find:

```rust
    /// Print transcribed text to stdout instead of typing it
    #[arg(long)]
    dry_run: bool,
```

Add before it:

```rust
    /// Enable GPU (Vulkan) inference
    #[arg(long)]
    gpu: bool,
```

**Step 2: Wire the flag into config**

In `main()`, find the block where CLI args are merged into config (after `config.silence_threshold_ms = args.silence_ms;`). Add:

```rust
    if args.gpu {
        config.use_gpu = true;
    }
```

**Step 3: Add startup log line**

In `main()`, find:

```rust
    if config.dry_run {
        info!("Dry-run mode: text will be printed to stdout");
    }
```

Add after it:

```rust
    if config.use_gpu {
        info!("GPU inference: enabled (Vulkan)");
    } else {
        info!("GPU inference: disabled (CPU)");
    }
```

**Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: exits 0.

**Step 5: Smoke-test the flag**

Run: `cargo run -- --dry-run --gpu --log-level debug 2>&1 | head -20`
Expected: log line `GPU inference: enabled (Vulkan)` appears (then it will fail on missing model — that's fine for a smoke test).

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --gpu CLI flag and startup log for GPU status"
```

---

### Task 4: Pass `use_gpu` to WhisperContextParameters

**Files:**
- Modify: `src/transcriber.rs`

**Step 1: Update Transcriber::new**

In `src/transcriber.rs`, find:

```rust
        let ctx = WhisperContext::new_with_params(
            config.model_path.to_str().context("Invalid model path")?,
            WhisperContextParameters::default(),
        )
```

Replace with:

```rust
        let ctx = WhisperContext::new_with_params(
            config.model_path.to_str().context("Invalid model path")?,
            WhisperContextParameters {
                use_gpu: config.use_gpu,
                ..Default::default()
            },
        )
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: exits 0. (If `WhisperContextParameters` does not have a `use_gpu` field in this version of whisper-rs, see note below.)

> **Note:** If the build fails with "no field `use_gpu`", check the whisper-rs 0.11 API:
> ```bash
> cargo doc --open
> ```
> Look for `WhisperContextParameters`. The field may be named differently (e.g. `gpu_device`). Adjust the field name accordingly and update this plan.

**Step 3: Run the full test suite**

Run: `cargo test`
Expected: all tests pass.

**Step 4: Commit**

```bash
git add src/transcriber.rs
git commit -m "feat: pass use_gpu to WhisperContextParameters for Vulkan inference"
```

---

### Task 5: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Add the config row**

In `CLAUDE.md`, find the Configuration section table. Find the row:

```
- `ptt_key` — PTT key name (e.g. `"KEY_SPACE"`, `"KEY_CAPSLOCK"`); `null` = VAD mode (default)
```

Add after it:

```
- `use_gpu` — Use Vulkan GPU for Whisper inference; `false` = CPU only (default)
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: document use_gpu config option in CLAUDE.md"
```

---

### Task 6: Final verification

**Step 1: Full test suite**

Run: `cargo test`
Expected: all tests pass.

**Step 2: Format check**

Run: `cargo fmt --check`
Expected: exits 0.

**Step 3: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: exits 0, no warnings.

**Step 4: Dry-run smoke test (CPU mode)**

Run: `cargo run -- --dry-run 2>&1 | grep -E "GPU inference"`
Expected: `GPU inference: disabled (CPU)`

**Step 5: Dry-run smoke test (GPU flag)**

Run: `cargo run -- --dry-run --gpu 2>&1 | grep -E "GPU inference"`
Expected: `GPU inference: enabled (Vulkan)`
