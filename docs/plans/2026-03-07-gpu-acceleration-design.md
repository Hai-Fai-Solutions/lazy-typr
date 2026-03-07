# Design: Optional GPU Acceleration for Whisper Inference

**Date:** 2026-03-07
**Branch:** `feature/gpu-acceleration`
**Status:** Draft

---

## Problem

Whisper inference runs entirely on the CPU. On machines with a discrete GPU the transcription
latency is significantly higher than necessary. `whisper.cpp` supports CUDA and OpenCL backends;
`whisper-rs` 0.11 exposes both as Cargo features. Neither the build system nor the runtime
currently offer any way to enable them.

---

## Design

### 1. Build-time — Cargo features (`Cargo.toml`)

GPU backends are compiled into `whisper.cpp` at build time. Three feature flags are added; the
default build remains CPU-only:

```
cuda    → whisper-rs/cuda    (NVIDIA, requires CUDA Toolkit ≥ 12)
hipblas → whisper-rs/hipblas (AMD ROCm, requires ROCm ≥ 5.x)
vulkan  → whisper-rs/vulkan  (NVIDIA or AMD, requires Vulkan ICD + headers)
```

`opencl` was present in `whisper-rs` 0.11 but removed in 0.14 in favour of Vulkan.
`whisper-rs` is upgraded from 0.11 to 0.14 as part of this change — required because 0.11 bundled
an old `whisper.cpp` that hardcoded CUDA arch `compute_52`, which CUDA 12+ dropped.

Which backend whisper.cpp actually uses at runtime is controlled by `WhisperContextParameters`.

### 2. Config (`src/config.rs`)

Two new fields added to `Config`:

| Field | Type | Default | Serialised |
|---|---|---|---|
| `use_gpu` | `bool` | `false` | yes (`#[serde(default)]`) |
| `gpu_device` | `Option<i32>` | `None` | yes (`#[serde(default)]`) |

`#[serde(default)]` ensures existing `config.json` files without these keys deserialise without
error. `whisper-rs` 0.14 exposes `gpu_device` on `WhisperContextParameters`; `None` maps to
device index `0` (let whisper.cpp pick the first GPU).

### 3. Context initialisation (`src/transcriber.rs` — `Transcriber::new`)

`WhisperContextParameters` gains GPU settings before the context is created using struct literal
syntax with a spread to handle additional fields added in 0.14 (`flash_attn`, `dtw_parameters`):

```rust
WhisperContextParameters {
    use_gpu: config.use_gpu,
    gpu_device: config.gpu_device.unwrap_or(0),
    ..WhisperContextParameters::default()
}
```

`use_gpu: false` (the default) is a no-op — whisper.cpp ignores the GPU even if the binary
was compiled with a GPU feature. A binary compiled without any GPU feature ignores `use_gpu: true`
silently (whisper.cpp falls back to CPU; no error is raised).

`use_gpu: bool` is stored on the `Transcriber` struct so the inference method can read it
without re-consulting the config.

### 4. CPU thread reduction (`src/transcriber.rs` — `transcribe`)

When the GPU handles matrix multiply, CPU threads are only needed for pre/post-processing
(tokenisation, segment assembly). Using the full core count wastes scheduling overhead.

```
let threads = if self.use_gpu { 2 } else { num_cpus() };
params.set_n_threads(threads);
```

`num_cpus()` is already capped at 8 (see existing implementation). The GPU path uses a fixed
value of 2, which is sufficient for all current pre/post-processing work in whisper.cpp.

### 5. CLI (`src/main.rs`)

Two new optional flags:

| Flag | Maps to | Notes |
|---|---|---|
| `--gpu` | `config.use_gpu = true` | boolean, off by default |
| `--gpu-device <INDEX>` | `config.gpu_device = Some(n)` | integer, optional |

CLI overrides config file (same priority order as existing flags). A startup `info!` line
reports the effective GPU setting so users can confirm it is active:

```
info!("GPU acceleration: enabled (device {})", config.gpu_device.unwrap_or(0));
// or
info!("GPU acceleration: disabled (CPU only)");
```

### 6. Setup script (`setup.sh`)

GPU detection is added between the model-download step and the build step:

1. `lspci` is queried for NVIDIA or AMD GPU presence.
2. If a GPU is found the user is prompted: **Use GPU? [y/N]**.
3. On yes: the matching toolkit is installed (CUDA for NVIDIA, Vulkan for AMD) and
   `GPU_FEATURE` is set to `"cuda"` or `"vulkan"` respectively.
4. The build command becomes `cargo build --release --features "$GPU_FEATURE"` when
   `$GPU_FEATURE` is non-empty, or `cargo build --release` otherwise.
5. The generated `config.json` includes `"use_gpu": true/false` and `"gpu_device": null`
   reflecting the choice made.

If `lspci` is absent the script skips GPU detection entirely and proceeds with a CPU build.

### 7. README (`README.md`)

Five additions, no existing content removed:

1. **Features list** — one new bullet for GPU acceleration.
2. **Build section** — table of GPU feature flags with per-vendor build commands and
   build-time package requirements.
3. **CLI options table** — `--gpu` and `--gpu-device` rows.
4. **Configuration table** — `use_gpu` and `gpu_device` rows; example JSON updated.
5. **Troubleshooting** — entry for "GPU not used even with `--gpu`" explaining the
   build-time feature requirement.

---

## Error handling

| Situation | Behaviour |
|---|---|
| `use_gpu: true` but binary built without any GPU feature | whisper.cpp silently falls back to CPU; no error. Log line still says "GPU acceleration: enabled" — this is a known limitation, not a bug. |
| `use_gpu: true`, GPU feature compiled in, but no compatible GPU driver at runtime | `WhisperContext::new_with_params` returns an error → startup fails with `"Failed to load Whisper model"` + anyhow chain |
| `gpu_device` index out of range | whisper.cpp returns an error, propagated via the same `anyhow` chain |
| `lspci` absent in setup.sh | GPU detection block skipped; CPU build proceeds; no error |

---

## Security considerations

### New attack surface introduced

- **`gpu_device` field in `config.json`** — an `i32` index passed directly to whisper.cpp.
  An attacker who can write `config.json` could supply a large or negative index. whisper.cpp
  performs its own bounds check and returns an error; the value is never used in pointer
  arithmetic in Rust code. Risk: **low** (already requires write access to user config dir).

- **`lspci` invocation in `setup.sh`** — called without arguments; output is parsed with
  `grep -qi`. No user-supplied data is interpolated into the command. Risk: **none**.

- **`GPU_FEATURE` variable in `setup.sh`** — set only to the literals `"cuda"` or `"vulkan"`;
  never derived from user input or `lspci` output. Passed to `cargo build --features`.
  Risk: **none**.

### No change to existing threat surfaces

- Whisper output sanitisation path (`transcriber.rs` → `typer.rs`) is unchanged.
- `wtype` / `xdotool` invocation (`typer.rs`) is unchanged.
- Audio buffer handling is unchanged.
- `model_path` handling is unchanged.

---

## Testing

### Unit tests (no hardware required)

**`src/config.rs`**
- `test_default_use_gpu_is_false` — `Config::default().use_gpu == false`
- `test_default_gpu_device_is_none` — `Config::default().gpu_device.is_none()`
- `test_deserialization_missing_gpu_fields_defaults` — JSON without `use_gpu` / `gpu_device`
  deserialises without error and yields the defaults above
- Extend `test_serialization_roundtrip` — set `use_gpu: true, gpu_device: Some(1)`, round-trip
  through JSON, assert values survive

**`src/transcriber.rs`**
- `test_thread_count_cpu_mode` — construct a mock `Config` with `use_gpu: false`; verify
  `num_cpus()` path is taken (indirectly via the existing `test_num_cpus_in_valid_range`)
- `test_thread_count_gpu_mode` — with `use_gpu: true` the thread count must be `2` (test the
  helper or expose a `gpu_thread_count()` fn under `#[cfg(test)]`)

### Integration / CI

- All existing tests (`vad_pipeline`, `config_integration`, `ptt_key_coverage`) must pass
  unchanged — GPU fields default to `false`/`None`, so no GPU hardware is needed.
- `cargo clippy -- -D warnings` must pass with the two new `cfg`-free fields.
- `cargo build --release` (CPU-only, no feature flags) must succeed on the CI runner.

### Manual / hardware tests (out of scope for CI)

- Build with `--features cuda`, run `whisper-type --gpu --dry-run`, confirm log line
  `"GPU acceleration: enabled (device 0)"` appears.
- Build with `--features cuda`, run without `--gpu`, confirm CPU path is taken.
- Build without GPU features, run with `--gpu`, confirm fallback to CPU (no crash).

---

## Files changed

| File | Change |
|---|---|
| `Cargo.toml` | Add `[features]` block (`cuda`, `hipblas`, `vulkan`); upgrade `whisper-rs` 0.11 → 0.14 |
| `src/config.rs` | Add `use_gpu: bool`, `gpu_device: Option<i32>`; update `Default`; update tests |
| `src/transcriber.rs` | Store `use_gpu` on struct; GPU params via struct literal in `new()`; conditional thread count in `transcribe()` |
| `src/main.rs` | Add `--gpu`, `--gpu-device` CLI args; merge into config; add startup log line |
| `setup.sh` | GPU detection, conditional toolkit install, conditional feature build, updated config template |
| `README.md` | Features, build table, CLI options, config table, troubleshooting |
