# Plan: Optional GPU acceleration for Whisper inference

## Context

Whisper inference is currently CPU-only. `whisper.cpp` (and by extension `whisper-rs`) supports
several GPU backends compiled in at build time and activated per-context at runtime. This plan
adds optional GPU support behind Cargo feature flags so CPU mode remains the default and existing
users are unaffected.

---

## Branch

```bash
git checkout main
git checkout -b feature/gpu-acceleration
```

---

## Supported GPU backends

| Feature flag | Hardware | Build-time requirement |
|---|---|---|
| `cuda` | NVIDIA | CUDA Toolkit >= 12, `nvcc` on `$PATH` |
| `hipblas` | AMD (ROCm) | ROCm >= 5.x, `hipcc` on `$PATH` |
| `vulkan` | NVIDIA or AMD | Vulkan ICD + headers |

These are the GPU backends exposed by `whisper-rs` 0.14 (the version in use). `opencl` was removed
from `whisper.cpp` in favour of Vulkan and is no longer available.

CPU mode is the default — no feature flag = pure CPU, identical to current behaviour.

---

## Changes

### 1. [Cargo.toml](../../Cargo.toml)

Add `[features]` section:

```toml
[features]
default = []
cuda    = ["whisper-rs/cuda"]
hipblas = ["whisper-rs/hipblas"]
vulkan  = ["whisper-rs/vulkan"]
```

Also upgrade `whisper-rs` from `0.11` to `0.14`. This is required for CUDA 12+ support: 0.11
bundled an old `whisper.cpp` that hardcoded CUDA arch `compute_52`, which CUDA 12+ dropped. 0.14
targets arch >= 60 (Pascal) and exposes `hipblas` and `vulkan` backends.

---

### 2. [src/config.rs](../../src/config.rs)

Add two new fields to `Config`:

```rust
/// Use GPU for Whisper inference.
/// Requires the binary to have been built with --features cuda / hipblas / vulkan.
#[serde(default)]
pub use_gpu: bool,

/// GPU device index (0 = first GPU). None = let whisper.cpp pick.
#[serde(default)]
pub gpu_device: Option<i32>,
```

Add corresponding defaults in `impl Default for Config`:

```rust
use_gpu: false,
gpu_device: None,
```

Both fields use `#[serde(default)]` so existing `config.json` files without them keep working.

Add the fields to the serialisation round-trip test in the existing `test_serialization_roundtrip`
test (set `use_gpu: true`, `gpu_device: Some(0)`) and add a new test
`test_deserialization_missing_gpu_fields_defaults` that verifies the fields default to
`false` / `None` when absent from JSON.

---

### 3. [src/transcriber.rs](../../src/transcriber.rs)

Two changes: GPU context parameters and thread count reduction when GPU is active.

#### 3a. `Transcriber::new` — pass GPU params to `WhisperContext`

Replace:

```rust
let ctx = WhisperContext::new_with_params(
    config.model_path.to_str().context("Invalid model path")?,
    WhisperContextParameters::default(),
)
.context("Failed to load Whisper model")?;
```

With:

```rust
let ctx = WhisperContext::new_with_params(
    config.model_path.to_str().context("Invalid model path")?,
    WhisperContextParameters {
        use_gpu: config.use_gpu,
        gpu_device: config.gpu_device.unwrap_or(0),
        ..WhisperContextParameters::default()
    },
)
.context("Failed to load Whisper model")?;
```

The `..WhisperContextParameters::default()` spread is required because `whisper-rs` 0.14 added
additional fields (`flash_attn`, `dtw_parameters`) that we do not need to set.

Store `use_gpu` on the struct so `transcribe()` can read it:

```rust
pub struct Transcriber {
    ctx: WhisperContext,
    language: String,
    use_gpu: bool,       // ← add
}
```

And in `Ok(Self { ... })`:

```rust
use_gpu: config.use_gpu,
```

#### 3b. `transcribe()` — reduce CPU threads when GPU handles compute

Replace the unconditional:

```rust
params.set_n_threads(num_cpus());
```

With:

```rust
// When GPU handles inference, 1-2 CPU threads are enough for pre/post-processing.
// On CPU-only builds, use all available cores (capped at 8).
let threads = if self.use_gpu { 2 } else { num_cpus() };
params.set_n_threads(threads);
```

---

### 4. [src/main.rs](../../src/main.rs)

Add two CLI arguments to `Args`:

```rust
/// Enable GPU acceleration (binary must be built with --features cuda/vulkan/hipblas/opencl)
#[arg(long)]
gpu: bool,

/// GPU device index to use (default: 0, i.e. first GPU)
#[arg(long)]
gpu_device: Option<i32>,
```

Merge into config alongside the other CLI overrides:

```rust
if args.gpu {
    config.use_gpu = true;
}
if let Some(d) = args.gpu_device {
    config.gpu_device = Some(d);
}
```

Add a startup log line after the existing `info!("Language: ...")`:

```rust
if config.use_gpu {
    info!("GPU acceleration: enabled (device {})", config.gpu_device.unwrap_or(0));
} else {
    info!("GPU acceleration: disabled (CPU only)");
}
```

---

### 5. [setup.sh](../../setup.sh)

After the model download section and before the build step, add GPU detection and an opt-in prompt:

```bash
# --- GPU acceleration (optional) ---
GPU_FEATURE=""

detect_gpu_vendor() {
    if lspci 2>/dev/null | grep -qi "nvidia"; then
        echo "nvidia"
    elif lspci 2>/dev/null | grep -qi "amd\|radeon"; then
        echo "amd"
    else
        echo "none"
    fi
}

GPU_VENDOR=$(detect_gpu_vendor)

if [ "$GPU_VENDOR" != "none" ]; then
    echo ""
    echo "GPU detected (${GPU_VENDOR}). Enable GPU acceleration? (faster inference)"
    read -rp "Use GPU? [y/N]: " USE_GPU
    if [[ "${USE_GPU,,}" == "y" ]]; then
        if [ "$GPU_VENDOR" = "nvidia" ]; then
            GPU_FEATURE="cuda"
            if [ "$DISTRO" = "arch" ]; then
                log "Installing CUDA toolkit..."
                sudo pacman -S --noconfirm cuda
            elif [ "$DISTRO" = "debian" ]; then
                log "Installing CUDA toolkit..."
                sudo apt-get install -y nvidia-cuda-toolkit
            fi
        elif [ "$GPU_VENDOR" = "amd" ]; then
            GPU_FEATURE="vulkan"
            if [ "$DISTRO" = "arch" ]; then
                log "Installing Vulkan support for AMD..."
                sudo pacman -S --noconfirm vulkan-icd-loader vulkan-radeon
            elif [ "$DISTRO" = "debian" ]; then
                log "Installing Vulkan support for AMD..."
                sudo apt-get install -y libvulkan-dev mesa-vulkan-drivers
            fi
        fi
        ok "GPU feature: ${GPU_FEATURE}"
    fi
fi
```

Change the build command:

```bash
# Before:
cargo build --release

# After:
if [ -n "$GPU_FEATURE" ]; then
    log "Building with GPU support (--features ${GPU_FEATURE})..."
    cargo build --release --features "$GPU_FEATURE"
else
    log "Building (CPU only)..."
    cargo build --release
fi
```

Update the config written by setup.sh to include the new fields:

```bash
cat > "$CONFIG_DIR/config.json" << EOF
{
  "model_path": "$MODEL_PATH",
  "device_name": null,
  "language": "de",
  "silence_threshold_ms": 800,
  "min_speech_ms": 300,
  "max_buffer_secs": 30.0,
  "vad_threshold": 0.01,
  "use_gpu": $([ -n "$GPU_FEATURE" ] && echo "true" || echo "false"),
  "gpu_device": null
}
EOF
```

---

### 6. [README.md](../../README.md)

#### 6a. Features list (top section)

Add one bullet after the existing `⚡ Multi-threaded` line:

```markdown
- ⚡ **GPU acceleration** — optional NVIDIA (CUDA), AMD (ROCm/Vulkan), or any Vulkan/OpenCL GPU
```

#### 6b. Build section — add GPU variants

After the existing `cargo build --release` block, add:

```markdown
#### GPU-accelerated builds

The binary must be compiled with the appropriate feature flag for your GPU:

| GPU | Feature flag | Build command |
|-----|---|---|
| NVIDIA | `cuda` | `cargo build --release --features cuda` |
| AMD (ROCm) | `hipblas` | `cargo build --release --features hipblas` |
| NVIDIA or AMD (Vulkan) | `vulkan` | `cargo build --release --features vulkan` |
| Any OpenCL GPU | `opencl` | `cargo build --release --features opencl` |

**Build-time requirements:**

- NVIDIA/CUDA: CUDA Toolkit >= 11.8 (`nvcc` on `$PATH`)
  - Arch: `sudo pacman -S cuda`
  - Debian/Ubuntu: `sudo apt install nvidia-cuda-toolkit`
- AMD/ROCm: ROCm >= 5.x (`hipcc` on `$PATH`)
- Vulkan: Vulkan SDK headers + ICD
  - Arch: `sudo pacman -S vulkan-icd-loader vulkan-radeon` (AMD) or `vulkan-nvidia` (NVIDIA)
  - Debian/Ubuntu: `sudo apt install libvulkan-dev mesa-vulkan-drivers`

The resulting binary only needs the GPU **runtime** (driver + shared libs) — no SDK required on
end-user machines.
```

#### 6c. Usage / CLI options table

Add two rows to the OPTIONS block:

```
    --gpu                     Enable GPU acceleration (build must include a GPU feature)
    --gpu-device <INDEX>      GPU device index [default: 0]
```

#### 6d. Configuration table

Add two rows:

| `use_gpu` | Enable GPU inference (binary must be built with a GPU feature flag) | `false` |
| `gpu_device` | GPU device index; `null` = let whisper.cpp choose | `null` |

Update the example `config.json` snippet to include:

```json
  "use_gpu": false,
  "gpu_device": null,
```

#### 6e. Troubleshooting section

Add a new entry:

```markdown
**GPU not used even with `--gpu`**

The binary must be compiled with the matching feature flag — the `--gpu` flag alone is not
sufficient if the binary was built without GPU support:

```bash
# Check which features the binary was compiled with:
whisper-type --help   # GPU options only appear when built with a GPU feature

# Rebuild with CUDA support:
cargo build --release --features cuda
```

Verify with: `whisper-type --gpu --dry-run --log-level debug`
— you should see `GPU acceleration: enabled` in the log output.
```

---

## Execution order

1. Create branch `feature/gpu-acceleration`
2. Add `[features]` block to `Cargo.toml`
3. Add `use_gpu` / `gpu_device` to `Config` + update tests
4. Update `Transcriber::new` and `transcribe()` in `transcriber.rs`
5. Add `--gpu` / `--gpu-device` CLI args and config merge in `main.rs`
6. Update `setup.sh` (GPU detection, conditional build, config template)
7. Update `README.md` (features, build table, CLI options, config table, troubleshooting)
8. `cargo fmt && cargo clippy -- -D warnings`
9. `cargo test` — all existing tests must pass (no GPU hardware required)
10. Open PR → `main`

---

## What does NOT change

- Audio pipeline (`audio/mod.rs`, `audio/vad.rs`) — unaffected
- Typer / PTT (`typer.rs`, `ptt.rs`) — unaffected
- All existing integration tests — GPU fields default to `false`/`None`, no hardware needed
- CPU mode behaviour — identical to current when `use_gpu = false`
