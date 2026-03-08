# GPU Device Selection — Design

**Date:** 2026-03-08
**Status:** Approved

## Problem

GPU inference (`--gpu` / `use_gpu`) always targets Vulkan device 0. On systems with multiple GPUs, users cannot direct inference to a specific device.

## Decisions

| Question | Answer |
|----------|--------|
| Identifier | Integer index (maps directly to whisper-rs `gpu_device: c_int`) |
| API surface | New `gpu_device: u32` config field + `--gpu-device <N>` CLI flag |
| Backwards compat | `--gpu` and `use_gpu` unchanged; new field is additive |
| Implicit enable | `--gpu-device N` implies `use_gpu = true` (no need to also pass `--gpu`) |
| Discovery | `--list-gpu-devices` prints index, name, and VRAM for each Vulkan device |
| Default | `gpu_device = 0` — same behaviour as today |

## Components

### 1. `src/config.rs` — new field

```rust
/// Vulkan GPU device index. 0 = first device. Only used when use_gpu = true.
#[serde(default)]
pub gpu_device: u32,
```

`#[serde(default)]` keeps existing `config.json` files working (missing key → `0`).

### 2. `src/main.rs` — CLI flags

```rust
/// GPU device index (implies --gpu)
#[arg(long, value_name = "N")]
gpu_device: Option<u32>,

/// List available Vulkan GPU devices and exit
#[arg(long)]
list_gpu_devices: bool,
```

Merge logic (after existing `--gpu` handling):

```rust
if let Some(dev) = args.gpu_device {
    config.use_gpu = true;
    config.gpu_device = dev;
}
```

`--list-gpu-devices` is handled early (before tracing init, like `--list-devices`):

```
GPU devices:
  0: AMD Radeon RX 6800 XT  (16376 MB total, 16376 MB free)
  1: Intel Arc A770          ( 8192 MB total,  7900 MB free)
```

Uses `whisper_rs::vulkan::list_devices()` (already available in whisper-rs 0.15).

### 3. `src/transcriber.rs` — pass device to context

```rust
WhisperContextParameters {
    use_gpu: config.use_gpu,
    gpu_device: config.gpu_device as i32,
    ..Default::default()
}
```

### 4. Logging

```
[INFO] GPU inference: enabled (Vulkan, device 1)
// or
[INFO] GPU inference: disabled (CPU)
```

### 5. Testing

New unit tests in `src/config.rs`:

- `test_default_gpu_device_is_zero`
- `test_gpu_device_round_trips_through_json`
- `test_gpu_device_absent_in_legacy_json_defaults_to_zero`

No new integration tests — transcriber path not reachable in `--dry-run` tests.

## Developer experience

```bash
# List available GPU devices
whisper-type --list-gpu-devices

# Use GPU device 0 (same as --gpu)
whisper-type --gpu

# Use GPU device 1
whisper-type --gpu-device 1

# Config file
{
  "use_gpu": true,
  "gpu_device": 1
}
```

## CLAUDE.md update

Add row to the Configuration table:

| `gpu_device` | Vulkan GPU device index (default: `0`); only used when `use_gpu` is `true` |
