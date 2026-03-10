# CUDA + Vulkan Backend Selection Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Auto-select CUDA when an NVIDIA GPU is detected, Vulkan otherwise, CPU as final fallback — with a `gpu_backend` config/CLI override.

**Architecture:** A new `src/gpu.rs` module owns `GpuBackend` (user-facing enum), `ResolvedBackend` (internal resolved enum), `detect_backend()` (nvml-wrapper probe), and `list_gpu_devices()`. `Config` replaces `use_gpu: bool` with `gpu_backend: GpuBackend`. `Transcriber` maps `ResolvedBackend` to `WhisperContextParameters`. whisper-rs is compiled with both `cuda` and `vulkan` features; whisper.cpp picks CUDA over Vulkan automatically when both are present, so `ResolvedBackend` is used for logging and fallback logic rather than runtime selection.

**Tech Stack:** Rust, whisper-rs 0.15 (`cuda` + `vulkan` features), nvml-wrapper 0.12, serde_json for config.

---

## Task 1: Add dependencies and update CI workflows

**Files:**
- Modify: `Cargo.toml`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`

**Step 1: Update Cargo.toml**

Change the whisper-rs line and add nvml-wrapper:

```toml
whisper-rs = { version = "0.15", features = ["vulkan", "cuda"] }
nvml-wrapper = "0.12"
```

**Step 2: Update CI workflows to install CUDA toolkit**

Both `ci.yml` (clippy + test jobs) and `release.yml` need the CUDA toolkit for compilation. Use the `Jimver/cuda-toolkit` GitHub Action. Add it **after** the Vulkan SDK step and **before** the Rust toolchain step in each job that runs `cargo`.

In `ci.yml`, add to the `clippy` job steps (after `Install Vulkan SDK`, before `Install Rust stable`):

```yaml
      - name: Install CUDA toolkit
        uses: Jimver/cuda-toolkit@v0.2.22
        with:
          cuda: '12.6.3'
          method: 'network'
          sub-packages: '["nvcc", "cudart-dev"]'
```

Repeat the same block in the `test` job and in `release.yml`'s `build-and-release` job.

Only `nvcc` and `cudart-dev` are needed — this avoids downloading the full ~3 GB CUDA suite. The `network` method fetches only what's requested.

**Step 3: Verify local build with CUDA toolkit installed**

```bash
cargo build 2>&1 | tail -5
```

Expected: build succeeds. (Prerequisite: `cuda` toolkit installed locally — `sudo pacman -S cuda` on Arch/CachyOS.)

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock .github/workflows/ci.yml .github/workflows/release.yml
git commit -m "chore: add cuda feature, nvml-wrapper, and CUDA toolkit in CI"
```

---

## Task 2: Create `src/gpu.rs` — types and serialization

**Files:**
- Create: `src/gpu.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing tests**

Create `src/gpu.rs` with the test module first:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GpuBackend {
    #[default]
    Auto,
    Cuda,
    Vulkan,
    Cpu,
}

/// Fully resolved backend — never Auto. Carries the device index.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedBackend {
    Cuda(u32),
    Vulkan(u32),
    Cpu,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_backend_default_is_auto() {
        assert_eq!(GpuBackend::default(), GpuBackend::Auto);
    }

    #[test]
    fn test_gpu_backend_serializes_to_snake_case() {
        assert_eq!(serde_json::to_string(&GpuBackend::Auto).unwrap(), r#""auto""#);
        assert_eq!(serde_json::to_string(&GpuBackend::Cuda).unwrap(), r#""cuda""#);
        assert_eq!(serde_json::to_string(&GpuBackend::Vulkan).unwrap(), r#""vulkan""#);
        assert_eq!(serde_json::to_string(&GpuBackend::Cpu).unwrap(), r#""cpu""#);
    }

    #[test]
    fn test_gpu_backend_deserializes_from_snake_case() {
        assert_eq!(serde_json::from_str::<GpuBackend>(r#""auto""#).unwrap(), GpuBackend::Auto);
        assert_eq!(serde_json::from_str::<GpuBackend>(r#""cuda""#).unwrap(), GpuBackend::Cuda);
        assert_eq!(serde_json::from_str::<GpuBackend>(r#""vulkan""#).unwrap(), GpuBackend::Vulkan);
        assert_eq!(serde_json::from_str::<GpuBackend>(r#""cpu""#).unwrap(), GpuBackend::Cpu);
    }

    #[test]
    fn test_gpu_backend_invalid_value_fails() {
        assert!(serde_json::from_str::<GpuBackend>(r#""GPU""#).is_err());
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test gpu 2>&1
```

Expected: compile error — module not in `lib.rs`.

**Step 3: Expose module in `src/lib.rs`**

Add to `src/lib.rs`:
```rust
pub mod gpu;
```

**Step 4: Run tests to verify they pass**

```bash
cargo test gpu::tests 2>&1
```

Expected: all 4 tests pass.

**Step 5: Commit**

```bash
git add src/gpu.rs src/lib.rs
git commit -m "feat: add GpuBackend and ResolvedBackend types"
```

---

## Task 3: Implement `detect_backend` in `src/gpu.rs`

**Files:**
- Modify: `src/gpu.rs`

**Step 1: Write the failing test**

Add to the `tests` module in `src/gpu.rs`:

```rust
#[test]
fn test_detect_cpu_returns_cpu() {
    assert_eq!(detect_backend(&GpuBackend::Cpu, 0), ResolvedBackend::Cpu);
}

#[test]
fn test_detect_cuda_explicit_returns_cuda_with_device() {
    assert_eq!(detect_backend(&GpuBackend::Cuda, 2), ResolvedBackend::Cuda(2));
}

#[test]
fn test_detect_vulkan_explicit_returns_vulkan_with_device() {
    assert_eq!(detect_backend(&GpuBackend::Vulkan, 1), ResolvedBackend::Vulkan(1));
}
```

Note: `Auto` cannot be unit-tested without hardware — it's covered by the integration of the module.

**Step 2: Run tests to verify they fail**

```bash
cargo test detect 2>&1
```

Expected: compile error — `detect_backend` not defined.

**Step 3: Implement `detect_backend`**

Add to `src/gpu.rs`, above the `#[cfg(test)]` block:

```rust
/// Resolve the configured backend to a concrete choice.
/// `Auto` probes for NVIDIA via nvml, then falls back to Vulkan, then CPU.
pub fn detect_backend(backend: &GpuBackend, device: u32) -> ResolvedBackend {
    match backend {
        GpuBackend::Cpu => ResolvedBackend::Cpu,
        GpuBackend::Cuda => ResolvedBackend::Cuda(device),
        GpuBackend::Vulkan => ResolvedBackend::Vulkan(device),
        GpuBackend::Auto => probe_auto(device),
    }
}

fn probe_auto(device: u32) -> ResolvedBackend {
    // Try NVIDIA via NVML
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if nvml.device_count().unwrap_or(0) > 0 {
            return ResolvedBackend::Cuda(device);
        }
    }
    // Try Vulkan
    let vulkan_devices = whisper_rs::vulkan::list_devices();
    if !vulkan_devices.is_empty() {
        return ResolvedBackend::Vulkan(device);
    }
    ResolvedBackend::Cpu
}
```

Add the import at the top of `src/gpu.rs`:

```rust
use serde::{Deserialize, Serialize};
```

**Step 4: Run tests to verify they pass**

```bash
cargo test detect 2>&1
```

Expected: all 3 tests pass.

**Step 5: Commit**

```bash
git add src/gpu.rs
git commit -m "feat: implement detect_backend with nvml NVIDIA probe"
```

---

## Task 4: Implement `list_gpu_devices` in `src/gpu.rs`

**Files:**
- Modify: `src/gpu.rs`

**Step 1: Implement**

Add to `src/gpu.rs` (no unit tests — this is I/O output):

```rust
/// Print available GPU devices to stdout.
/// Shows CUDA devices (via NVML) and Vulkan devices in separate sections.
/// Silently skips each section if the underlying library fails to initialize.
pub fn list_gpu_devices() {
    let mut any = false;

    // CUDA devices via NVML
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        let count = nvml.device_count().unwrap_or(0);
        if count > 0 {
            any = true;
            println!("CUDA devices (NVIDIA):");
            for i in 0..count {
                if let Ok(dev) = nvml.device_by_index(i) {
                    let name = dev.name().unwrap_or_else(|_| "unknown".to_string());
                    let mem = dev.memory_info();
                    match mem {
                        Ok(m) => println!(
                            "  {}: {}  ({} MB total, {} MB free)",
                            i,
                            name,
                            m.total / 1024 / 1024,
                            m.free / 1024 / 1024,
                        ),
                        Err(_) => println!("  {}: {}", i, name),
                    }
                }
            }
        }
    }

    // Vulkan devices
    let vulkan_devices = whisper_rs::vulkan::list_devices();
    if !vulkan_devices.is_empty() {
        any = true;
        println!("Vulkan devices:");
        for dev in &vulkan_devices {
            println!(
                "  {}: {}  ({} MB total, {} MB free)",
                dev.id,
                dev.name,
                dev.vram.total / 1024 / 1024,
                dev.vram.free / 1024 / 1024,
            );
        }
    }

    if !any {
        println!("No GPU devices found.");
    }
}
```

**Step 2: Verify it compiles**

```bash
cargo build 2>&1 | tail -5
```

Expected: no errors.

**Step 3: Commit**

```bash
git add src/gpu.rs
git commit -m "feat: implement list_gpu_devices with CUDA and Vulkan sections"
```

---

## Task 5: Update `Config` — replace `use_gpu` with `gpu_backend`

**Files:**
- Modify: `src/config.rs`

**Step 1: Write failing tests**

Add to `src/config.rs` tests module:

```rust
#[test]
fn test_default_gpu_backend_is_auto() {
    assert_eq!(Config::default().gpu_backend, crate::gpu::GpuBackend::Auto);
}

#[test]
fn test_gpu_backend_round_trips_through_json() {
    let cfg = Config {
        gpu_backend: crate::gpu::GpuBackend::Cuda,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.gpu_backend, crate::gpu::GpuBackend::Cuda);
}

#[test]
fn test_gpu_backend_absent_in_legacy_json_defaults_to_auto() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.gpu_backend, crate::gpu::GpuBackend::Auto);
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test config::tests::test_default_gpu_backend 2>&1
```

Expected: compile error — field does not exist.

**Step 3: Update Config struct**

In `src/config.rs`:

1. Add import at top:
```rust
use crate::gpu::GpuBackend;
```

2. Replace these two fields in `Config`:
```rust
// REMOVE:
pub use_gpu: bool,
pub gpu_device: u32,

// ADD:
#[serde(default)]
pub gpu_backend: GpuBackend,
/// Device index passed to whichever backend is active.
#[serde(default)]
pub gpu_device: u32,
```

3. Update `Config::default()`:
```rust
// REMOVE:
use_gpu: false,
gpu_device: 0,

// ADD:
gpu_backend: GpuBackend::default(),
gpu_device: 0,
```

**Step 4: Fix existing tests that reference `use_gpu`**

Search and update tests in `config.rs` that reference `use_gpu`:

```bash
grep -n "use_gpu" src/config.rs
```

Replace each occurrence:
- `config.use_gpu: bool` → remove (field gone)
- Test `test_default_use_gpu_is_false` → delete (superseded by `test_default_gpu_backend_is_auto`)
- Test `test_use_gpu_round_trips_through_json` → delete (superseded)
- Test `test_use_gpu_absent_in_legacy_json_defaults_to_false` → delete (superseded)

**Step 5: Run all config tests**

```bash
cargo test config::tests 2>&1
```

Expected: all tests pass.

**Step 6: Commit**

```bash
git add src/config.rs
git commit -m "feat: replace use_gpu with gpu_backend in Config"
```

---

## Task 6: Update `Transcriber` to accept `ResolvedBackend`

**Files:**
- Modify: `src/transcriber.rs`

**Step 1: Write failing test**

Add to `src/transcriber.rs` tests:

```rust
#[test]
fn test_whisper_ctx_params_cpu_disables_gpu() {
    use crate::gpu::ResolvedBackend;
    let params = backend_to_ctx_params(&ResolvedBackend::Cpu);
    assert!(!params.use_gpu);
}

#[test]
fn test_whisper_ctx_params_cuda_enables_gpu_with_device() {
    use crate::gpu::ResolvedBackend;
    let params = backend_to_ctx_params(&ResolvedBackend::Cuda(2));
    assert!(params.use_gpu);
    assert_eq!(params.gpu_device, 2);
}

#[test]
fn test_whisper_ctx_params_vulkan_enables_gpu_with_device() {
    use crate::gpu::ResolvedBackend;
    let params = backend_to_ctx_params(&ResolvedBackend::Vulkan(1));
    assert!(params.use_gpu);
    assert_eq!(params.gpu_device, 1);
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test transcriber::tests 2>&1
```

Expected: compile error — `backend_to_ctx_params` not defined.

**Step 3: Implement**

In `src/transcriber.rs`, add import and helper:

```rust
use crate::gpu::ResolvedBackend;
use whisper_rs::WhisperContextParameters;
```

Add helper function:

```rust
pub(crate) fn backend_to_ctx_params(backend: &ResolvedBackend) -> WhisperContextParameters<'static> {
    match backend {
        ResolvedBackend::Cpu => WhisperContextParameters {
            use_gpu: false,
            ..Default::default()
        },
        ResolvedBackend::Cuda(device) | ResolvedBackend::Vulkan(device) => {
            WhisperContextParameters {
                use_gpu: true,
                gpu_device: *device as std::ffi::c_int,
                ..Default::default()
            }
        }
    }
}
```

Update `Transcriber::new` signature and body:

```rust
// Change signature:
pub fn new(config: &Config, backend: &ResolvedBackend) -> Result<Self> {

// Replace WhisperContextParameters construction:
let ctx = WhisperContext::new_with_params(
    config.model_path.to_str().context("Invalid model path")?,
    backend_to_ctx_params(backend),
)
.context("Failed to load Whisper model")?;
```

**Step 4: Run tests**

```bash
cargo test transcriber::tests 2>&1
```

Expected: the 3 new param tests pass. Existing hallucination and num_cpus tests still pass.

**Step 5: Commit**

```bash
git add src/transcriber.rs
git commit -m "feat: transcriber accepts ResolvedBackend for GPU context params"
```

---

## Task 7: Update `main.rs` — CLI args, backend resolution, logging

**Files:**
- Modify: `src/main.rs`

**Step 1: Update CLI args struct**

Replace the GPU-related args in the `Args` struct:

```rust
// REMOVE:
/// Enable GPU (Vulkan) inference
#[arg(long)]
gpu: bool,

// KEEP gpu_device and list_gpu_devices, ADD:
/// GPU backend: auto (default), cuda, vulkan, cpu
#[arg(long, value_name = "BACKEND")]
gpu_backend: Option<String>,

/// Enable GPU inference (shorthand for --gpu-backend auto)
#[arg(long)]
gpu: bool,
```

**Step 2: Update config merge block**

Replace the GPU config merge section:

```rust
// REMOVE old gpu block:
if args.gpu {
    config.use_gpu = true;
}
if let Some(dev) = args.gpu_device {
    config.use_gpu = true;
    config.gpu_device = dev;
}

// ADD new block:
if let Some(backend_str) = args.gpu_backend {
    config.gpu_backend = match backend_str.as_str() {
        "auto" => GpuBackend::Auto,
        "cuda" => GpuBackend::Cuda,
        "vulkan" => GpuBackend::Vulkan,
        "cpu" => GpuBackend::Cpu,
        other => {
            eprintln!("Error: unknown gpu-backend '{}'. Use: auto, cuda, vulkan, cpu", other);
            std::process::exit(1);
        }
    };
} else if args.gpu {
    // --gpu is a no-op now (Auto is already the default) but kept for compat
    config.gpu_backend = GpuBackend::Auto;
}
if let Some(dev) = args.gpu_device {
    config.gpu_device = dev;
}
```

Add import at top of `main.rs`:
```rust
use whisper_type::gpu::{self, GpuBackend};
```

**Step 3: Update `--list-gpu-devices` handler**

```rust
// REPLACE:
if args.list_gpu_devices {
    let devices = whisper_rs::vulkan::list_devices();
    if devices.is_empty() {
        println!("No Vulkan GPU devices found.");
    } else {
        println!("GPU devices:");
        for dev in &devices {
            println!(
                "  {}: {}  ({} MB total, {} MB free)",
                dev.id, dev.name,
                dev.vram.total / 1024 / 1024,
                dev.vram.free / 1024 / 1024,
            );
        }
    }
    return Ok(());
}

// WITH:
if args.list_gpu_devices {
    gpu::list_gpu_devices();
    return Ok(());
}
```

**Step 4: Resolve backend and update startup logging**

After the config merge block, add:

```rust
let resolved = gpu::detect_backend(&config.gpu_backend, config.gpu_device);
```

Replace the GPU logging block:

```rust
// REMOVE:
if config.use_gpu {
    info!("GPU inference: enabled (Vulkan, device {})", config.gpu_device);
} else {
    info!("GPU inference: disabled (CPU)");
}

// ADD:
match &resolved {
    whisper_type::gpu::ResolvedBackend::Cuda(dev) => {
        info!("GPU: cuda (device {})", dev);
    }
    whisper_type::gpu::ResolvedBackend::Vulkan(dev) => {
        info!("GPU: vulkan (device {})", dev);
    }
    whisper_type::gpu::ResolvedBackend::Cpu => {
        info!("GPU: cpu");
    }
}
```

**Step 5: Pass `resolved` to `Transcriber::new`**

```rust
// CHANGE:
let mut transcriber = match Transcriber::new(&config_t) {

// TO:
let resolved_t = resolved.clone();
// ... (resolved must be moved into the spawn closure)

// In the spawn closure, change:
let mut transcriber = match Transcriber::new(&config_t, &resolved_t) {
```

Full updated transcriber spawn:

```rust
let config_t = config.clone();
let resolved_t = resolved.clone();
let text_tx_t = text_tx.clone();
let transcriber_handle = std::thread::spawn(move || {
    let mut transcriber = match Transcriber::new(&config_t, &resolved_t) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to initialize Whisper: {}", e);
            std::process::exit(1);
        }
    };
    // ... rest unchanged
```

**Step 6: Build and run dry-run smoke test**

```bash
cargo build 2>&1 | tail -10
cargo run -- --dry-run --gpu-backend cpu 2>&1 | head -20
```

Expected: builds clean; dry-run starts and logs `GPU: cpu`.

**Step 7: Run full test suite**

```bash
cargo test 2>&1
```

Expected: all tests pass.

**Step 8: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --gpu-backend CLI flag, auto-detect CUDA/Vulkan/CPU at startup"
```

---

## Task 8: Update `CLAUDE.md` config table

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Update the Configuration section**

In `CLAUDE.md`, under "Configuration", replace the `use_gpu` row:

```markdown
# REMOVE:
- `use_gpu` — Use Vulkan GPU for Whisper inference; `false` = CPU only (default)
- `gpu_device` — Vulkan device index to use when `use_gpu` is true (default: `0`)

# ADD:
- `gpu_backend` — GPU backend: `"auto"` (default), `"cuda"`, `"vulkan"`, `"cpu"`. Auto detects NVIDIA → CUDA, else Vulkan, else CPU.
- `gpu_device` — Device index for the active GPU backend (default: `0`)
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for gpu_backend config field"
```

---

## Task 9: Final verification

**Step 1: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Expected: no warnings.

**Step 2: Run full test suite**

```bash
cargo test 2>&1
```

Expected: all tests pass.

**Step 3: Check `--help` output looks right**

```bash
cargo run -- --help 2>&1
```

Verify `--gpu-backend` appears with description, `--gpu` is still listed.

**Step 4: Check `--list-gpu-devices` output compiles and runs**

```bash
cargo run -- --list-gpu-devices 2>&1
```

Expected: shows CUDA and/or Vulkan sections (or "No GPU devices found.") without panicking.
