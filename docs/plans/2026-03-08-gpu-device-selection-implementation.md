# GPU Device Selection — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `gpu_device: u32` config field and `--gpu-device <N>` CLI flag so users can choose which Vulkan GPU whisper-rs uses for inference, plus a `--list-gpu-devices` flag for discovery.

**Architecture:** Add `gpu_device: u32` (serde-defaulted to `0`) to `Config`. Add `--gpu-device <N>` (implies `--gpu`) and `--list-gpu-devices` CLI flags. Pass both `use_gpu` and `gpu_device` into `WhisperContextParameters` in `Transcriber::new`. Update startup log to include device index when GPU is enabled.

**Tech Stack:** Rust, whisper-rs 0.15 (`WhisperContextParameters::gpu_device: c_int`, `whisper_rs::vulkan::list_devices()`), clap 4, serde_json

---

### Task 1: Add `gpu_device` to Config

**Files:**
- Modify: `src/config.rs`

**Step 1: Write the failing tests**

In `src/config.rs`, inside the `#[cfg(test)]` `tests` module, add after `test_default_use_gpu_is_false`:

```rust
#[test]
fn test_default_gpu_device_is_zero() {
    assert_eq!(Config::default().gpu_device, 0);
}

#[test]
fn test_gpu_device_round_trips_through_json() {
    let cfg = Config {
        gpu_device: 2,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.gpu_device, 2);
}

#[test]
fn test_gpu_device_absent_in_legacy_json_defaults_to_zero() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.gpu_device, 0);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_default_gpu_device_is_zero test_gpu_device_round_trips_through_json test_gpu_device_absent_in_legacy_json_defaults_to_zero -- --nocapture`
Expected: compile error — `gpu_device` field does not exist yet.

**Step 3: Add the field to the struct**

In `src/config.rs`, find:

```rust
    /// Use GPU (Vulkan) for Whisper inference. Default: false (CPU).
    #[serde(default)]
    pub use_gpu: bool,
```

Add after it:

```rust
    /// Vulkan GPU device index. 0 = first device. Only used when use_gpu = true.
    #[serde(default)]
    pub gpu_device: u32,
```

**Step 4: Add the default value**

In `impl Default for Config`, find:

```rust
            use_gpu: false,
```

Add after it:

```rust
            gpu_device: 0,
```

**Step 5: Run tests to verify they pass**

Run: `cargo test test_default_gpu_device_is_zero test_gpu_device_round_trips_through_json test_gpu_device_absent_in_legacy_json_defaults_to_zero -- --nocapture`
Expected: all 3 PASS.

**Step 6: Run full test suite**

Run: `cargo test`
Expected: all tests pass.

**Step 7: Commit**

```bash
git add src/config.rs
git commit -m "feat: add gpu_device field to Config (default: 0)"
```

---

### Task 2: Add `--gpu-device` and `--list-gpu-devices` CLI flags

**Files:**
- Modify: `src/main.rs`

**Step 1: Add the flags to the Args struct**

In `src/main.rs`, find:

```rust
    /// Enable GPU (Vulkan) inference
    #[arg(long)]
    gpu: bool,
```

Add after it:

```rust
    /// GPU device index to use for Vulkan inference (implies --gpu)
    #[arg(long, value_name = "N")]
    gpu_device: Option<u32>,

    /// List available Vulkan GPU devices and exit
    #[arg(long)]
    list_gpu_devices: bool,
```

**Step 2: Handle `--list-gpu-devices` early**

In `main()`, find:

```rust
    // List devices mode (no logging needed)
    if args.list_devices {
        audio::list_devices()?;
        return Ok(());
    }
```

Add after it:

```rust
    if args.list_gpu_devices {
        let devices = whisper_rs::vulkan::list_devices();
        if devices.is_empty() {
            println!("No Vulkan GPU devices found.");
        } else {
            println!("GPU devices:");
            for dev in &devices {
                println!(
                    "  {}: {}  ({} MB total, {} MB free)",
                    dev.id,
                    dev.name,
                    dev.vram.total / 1024 / 1024,
                    dev.vram.free / 1024 / 1024,
                );
            }
        }
        return Ok(());
    }
```

**Step 3: Wire `--gpu-device` into config**

In `main()`, find:

```rust
    if args.gpu {
        config.use_gpu = true;
    }
```

Add after it:

```rust
    if let Some(dev) = args.gpu_device {
        config.use_gpu = true;
        config.gpu_device = dev;
    }
```

**Step 4: Update the startup log**

In `main()`, find:

```rust
    if config.use_gpu {
        info!("GPU inference: enabled (Vulkan)");
    } else {
        info!("GPU inference: disabled (CPU)");
    }
```

Replace with:

```rust
    if config.use_gpu {
        info!("GPU inference: enabled (Vulkan, device {})", config.gpu_device);
    } else {
        info!("GPU inference: disabled (CPU)");
    }
```

**Step 5: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: exits 0.

**Step 6: Smoke-test `--list-gpu-devices`**

Run: `cargo run -- --list-gpu-devices`
Expected: either lists devices or prints "No Vulkan GPU devices found." — no panic.

**Step 7: Smoke-test `--gpu-device` implies GPU**

Run: `cargo run -- --dry-run --gpu-device 0 --log-level debug 2>&1 | grep -E "GPU inference"`
Expected: `GPU inference: enabled (Vulkan, device 0)`

**Step 8: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --gpu-device and --list-gpu-devices CLI flags"
```

---

### Task 3: Pass `gpu_device` to WhisperContextParameters

**Files:**
- Modify: `src/transcriber.rs`

**Step 1: Update `Transcriber::new`**

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
                gpu_device: config.gpu_device as i32,
                ..Default::default()
            },
        )
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: exits 0.

**Step 3: Run full test suite**

Run: `cargo test`
Expected: all tests pass.

**Step 4: Commit**

```bash
git add src/transcriber.rs
git commit -m "feat: pass gpu_device to WhisperContextParameters"
```

---

### Task 4: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Add the config row**

In `CLAUDE.md`, find:

```
- `use_gpu` — Use Vulkan GPU for Whisper inference; `false` = CPU only (default)
```

Add after it:

```
- `gpu_device` — Vulkan GPU device index (default: `0`); only used when `use_gpu` is `true`
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: document gpu_device config option in CLAUDE.md"
```

---

### Task 5: Final verification

**Step 1: Full test suite**

Run: `cargo test`
Expected: all tests pass.

**Step 2: Format check**

Run: `cargo fmt --check`
Expected: exits 0.

**Step 3: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: exits 0, no warnings.

**Step 4: Smoke-test CPU mode**

Run: `cargo run -- --dry-run 2>&1 | grep "GPU inference"`
Expected: `GPU inference: disabled (CPU)`

**Step 5: Smoke-test GPU mode with device**

Run: `cargo run -- --dry-run --gpu-device 0 2>&1 | grep "GPU inference"`
Expected: `GPU inference: enabled (Vulkan, device 0)`

**Step 6: Smoke-test `--gpu` still works**

Run: `cargo run -- --dry-run --gpu 2>&1 | grep "GPU inference"`
Expected: `GPU inference: enabled (Vulkan, device 0)`
