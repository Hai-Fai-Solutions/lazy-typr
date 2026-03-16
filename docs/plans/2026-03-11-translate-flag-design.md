# Design: Replace `--whisper-task` with `--translate` Flag

**Date:** 2026-03-11
**Issue:** ISSUE-2 — Silent fallback on `--whisper-task` typo
**Decision:** Breaking change; no backward compatibility required.

## Problem

`--whisper-task transcribe|translate` accepts a string enum value. A typo (e.g. `trnascribe`) can silently fall back without a clear user-facing error. Whisper only has two task variants — a string enum is the wrong type for a binary choice.

## Solution

Replace the `Task` enum and `whisper_task` config field with a single `translate: bool`. This eliminates ISSUE-2 entirely: a boolean flag either appears or it doesn't, leaving nothing to mistype.

## Architecture

### Config (`src/config.rs`)

- Remove `Task` enum
- Replace `pub whisper_task: Task` with `pub translate: bool`
- Default: `false` (transcribe)
- Serde: `#[serde(default)]` — missing field in `config.json` defaults to `false`
- Config file field renamed from `"whisper_task"` to `"translate"`
- Replace `apply_whisper_task_override(task: Option<Task>)` with `apply_translate_override(translate: bool)`

```rust
pub fn apply_translate_override(&mut self, translate: bool) {
    if translate {
        self.translate = true;
    }
}
```

Override semantics: only overrides when `true`. Users disable translation by updating `config.json`.

### CLI (`src/main.rs`)

- Remove `--whisper-task` arg
- Add `--translate` as a bare boolean presence flag

```rust
/// Translate speech to English instead of transcribing in the source language
#[arg(long)]
translate: bool,
```

- Present → `true`, absent → `false`
- No `--no-translate` flag needed

### Transcriber (`src/transcriber.rs`)

- Replace `task: Task` field with `translate: bool`
- `params.set_translate(self.translate)` — no change in behavior

### Startup log (`src/main.rs`)

Simplifies from a `match` block to:

```rust
info!("Translate: {}", config.translate);
```

## Config File

```json
{ "translate": false }
```

Old field `"whisper_task"` is dropped. Existing configs that had `"whisper_task": "translate"` must be manually updated to `"translate": true`.

## Testing

### Remove

- `Task` enum serde tests (`test_task_*`)
- `test_apply_whisper_task_override_*`

### Add / Update

| Location | Test |
|---|---|
| `config.rs` | `translate` field missing in JSON → `false` |
| `config.rs` | `"translate": true` roundtrip |
| `config.rs` | `"translate": false` roundtrip |
| `config.rs` | `apply_translate_override(false)` keeps existing value |
| `config.rs` | `apply_translate_override(true)` sets to `true` |
| `tests/cli_config_merge.rs` | No `--translate` flag keeps config value |
| `tests/cli_config_merge.rs` | `--translate` flag overrides config to `true` |
