# Design: Native GPU Backend Selection (CUDA + Vulkan)

**Date:** 2026-03-10
**Branch:** `feature/enable-cuda`
**Status:** Approved

## Goal

Automatically select the best available GPU backend for Whisper inference:
NVIDIA card present → CUDA; otherwise → Vulkan; no GPU → CPU.
User can override via config or CLI.

## Decisions

| Question | Decision |
|----------|----------|
| Selection strategy | Auto-detect at runtime, config/CLI override |
| Compiled backends | Always both (`vulkan` + `cuda` features in whisper-rs) |
| NVIDIA detection | `nvml-wrapper` crate |
| Fallback chain | CUDA → Vulkan → CPU |

## Data Model

### `GpuBackend` enum (new, in `src/gpu.rs`)

```rust
pub enum GpuBackend { Auto, Cuda, Vulkan, Cpu }
```

Serializes as `"auto"` / `"cuda"` / `"vulkan"` / `"cpu"`. Default: `Auto`.

### `ResolvedBackend` enum (internal, in `src/gpu.rs`)

```rust
pub enum ResolvedBackend { Cuda(u32), Vulkan(u32), Cpu }
```

Carries the device index. Never has an `Auto` variant — always fully resolved before use.

### Config changes (`src/config.rs`)

- **Remove** `use_gpu: bool`
- **Keep** `gpu_device: u32` (device index for whichever backend is active)
- **Add** `gpu_backend: GpuBackend` (default `Auto`)

Backward compat: old JSON with `"use_gpu": true/false` and no `gpu_backend` key
→ serde default kicks in → `Auto`. Existing users with NVIDIA hardware automatically
get CUDA; others keep Vulkan or CPU as before.

## New Module: `src/gpu.rs`

Owns all GPU concerns. Public surface:

```rust
pub fn detect_backend(backend: &GpuBackend, device: u32) -> ResolvedBackend
pub fn list_gpu_devices()
```

### `detect_backend` logic

```
Cpu    → ResolvedBackend::Cpu
Cuda   → ResolvedBackend::Cuda(device)      // trust the user, no probe
Vulkan → ResolvedBackend::Vulkan(device)    // trust the user, no probe
Auto   → nvml::Nvml::init() + device_count > 0?
             yes → ResolvedBackend::Cuda(device)
             no  → whisper_rs::vulkan::list_devices() non-empty?
                       yes → ResolvedBackend::Vulkan(device)
                       no  → ResolvedBackend::Cpu
```

### `list_gpu_devices` output

```
CUDA devices (NVIDIA):
  0: RTX 4090  (24576 MB total, 22000 MB free)

Vulkan devices:
  0: NVIDIA GeForce RTX 4090  (24576 MB total, 22000 MB free)
```

Each section is skipped gracefully if the respective library fails to init.

## CLI Changes (`src/main.rs`)

| Flag | Change |
|------|--------|
| `--gpu-backend <auto\|cuda\|vulkan\|cpu>` | New — explicit backend override |
| `--gpu` | Kept for backward compat; now a no-op (sets `Auto`, which is the default) |
| `--gpu-device N` | Unchanged — applies to whichever backend resolves |
| `--list-gpu-devices` | Now calls `gpu::list_gpu_devices()` (CUDA + Vulkan sections) |

Override priority (lowest → highest): `config.json` → `--gpu-backend` CLI flag.

## Transcriber Changes (`src/transcriber.rs`)

`Transcriber::new` receives `ResolvedBackend` instead of reading `use_gpu: bool` from config.

Mapping to `WhisperContextParameters`:
- `Cpu` → `use_gpu: false`
- `Vulkan(device)` → `use_gpu: true`, `gpu_device: device` (current behavior)
- `Cuda(device)` → `use_gpu: true`, CUDA-specific params (exact API confirmed during impl against whisper-rs 0.15)

## Startup Logging

Old: `"GPU inference: enabled (Vulkan, device 0)"`

New examples:
- `"GPU: cuda (device 0, NVIDIA GeForce RTX 4090, auto-detected)"`
- `"GPU: vulkan (device 0)"`
- `"GPU: cpu"`

## Cargo.toml Changes

```toml
whisper-rs = { version = "0.15", features = ["vulkan", "cuda"] }
nvml-wrapper = "0.10"
```

Build requirements: CUDA toolkit + Vulkan headers both required at build time.

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `cuda` feature, add `nvml-wrapper` |
| `src/gpu.rs` | New module: `GpuBackend`, `ResolvedBackend`, `detect_backend`, `list_gpu_devices` |
| `src/lib.rs` | Expose `pub mod gpu` |
| `src/config.rs` | Replace `use_gpu: bool` with `gpu_backend: GpuBackend`; keep `gpu_device` |
| `src/transcriber.rs` | Accept `ResolvedBackend`, map to `WhisperContextParameters` |
| `src/main.rs` | Add `--gpu-backend` flag, call `detect_backend`, update logging |
