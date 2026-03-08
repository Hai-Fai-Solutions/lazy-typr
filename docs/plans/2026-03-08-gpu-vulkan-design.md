# GPU Inference via Vulkan — Design

**Date:** 2026-03-08
**Status:** Approved

## Problem

Whisper inference runs entirely on CPU. On systems with a discrete GPU, inference latency is significantly higher than necessary. The goal is to allow users to opt in to GPU-accelerated inference via a cross-platform backend.

## Decisions

| Question | Answer |
|----------|--------|
| GPU backend | Vulkan — cross-platform, active in whisper.cpp, works on AMD/NVIDIA/Intel |
| Activation | Runtime opt-in: `use_gpu` in `config.json` and/or `--gpu` CLI flag |
| Default | `false` (CPU) — zero behaviour change for existing users |
| Compile-time | Always compiled in (no Cargo feature gate for users) |
| Fallback | whisper.cpp falls back to CPU if no Vulkan device is found — no crash |

## Components

### 1. `Cargo.toml`

```toml
whisper-rs = { version = "0.11", features = ["vulkan"] }
```

Enables the Vulkan/BLAS path in whisper.cpp at compile time. No new Rust crates needed.

### 2. `src/config.rs` — new field

```rust
/// Use GPU (Vulkan) for inference. Default: false (CPU).
#[serde(default)]
pub use_gpu: bool,
```

`#[serde(default)]` ensures existing `config.json` files without this key deserialize with `false`. Users set `"use_gpu": true` in `~/.config/whisper-type/config.json`.

### 3. `src/main.rs` — new CLI flag

```rust
/// Enable GPU (Vulkan) inference
#[arg(long)]
gpu: bool,
```

If `--gpu` is passed it overrides the config value. Override priority: `config.json` → `--gpu`.

### 4. `src/transcriber.rs` — pass flag to context

```rust
// Before
WhisperContextParameters::default()

// After
WhisperContextParameters {
    use_gpu: config.use_gpu,
    ..Default::default()
}
```

One-line change. whisper.cpp selects the Vulkan device and falls back to CPU with a warning if none is available.

### 5. Logging

On startup, emit at `info` level:

```
[INFO] GPU inference: enabled (Vulkan)
// or
[INFO] GPU inference: disabled (CPU)
```

### 6. Testing

- `config.rs` unit tests: `use_gpu` defaults to `false`, round-trips through JSON, absent key in legacy JSON deserializes cleanly.
- No integration test changes — `--dry-run` tests never reach the transcriber.

## Developer experience

1. Build normally — Vulkan is always linked.
2. To enable GPU: set `"use_gpu": true` in `~/.config/whisper-type/config.json`, or pass `--gpu` on the command line.
3. If no Vulkan device is present, whisper.cpp logs a warning and continues on CPU.

## CLAUDE.md update

Add row to the Configuration table:

| `use_gpu` | Use Vulkan GPU for Whisper inference (default: `false`) |
