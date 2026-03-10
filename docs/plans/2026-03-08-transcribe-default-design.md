# Transcribe-By-Default Task Selection — Design

**Date:** 2026-03-08
**Status:** Proposed

## Problem

User testing reports that spoken input appears to be translated by default. For dictation workflows, expected behavior is:

1. Default: transcribe speech in the spoken language.
2. Optional: translate only when explicitly configured.

Today, the app does not expose an explicit Whisper task configuration in runtime config/CLI, so task behavior is not user-visible or controllable.

## Goals

1. Make default behavior explicit and stable: `Transcribe`.
2. Allow opt-in translation via config and CLI.
3. Keep backward compatibility for existing config files.
4. Add regression tests so task behavior cannot silently drift.

## Decisions

| Question | Decision |
|----------|----------|
| Config surface | Add `whisper_task` to `Config` |
| Value type | Strong enum `Task { Transcribe, Translate }` |
| Serialization | `snake_case` serde strings: `transcribe`, `translate` |
| Default | `Task::Transcribe` |
| CLI override | Add `--whisper-task <transcribe|translate>` as `Option<Task>` |
| Merge precedence | CLI task overrides config only when explicitly provided |
| Runtime mapping | `Task::Transcribe` => `params.set_translate(false)`, `Task::Translate` => `params.set_translate(true)` |

## Components

### 1. `src/config.rs`

- Add:
  - `pub whisper_task: Task`
  - `Task` enum with serde + clap value support.
- Keep legacy config compatibility:
  - `#[serde(default)]` on `whisper_task`.
  - `impl Default for Task { Transcribe }`.
- Add helper:
  - `apply_whisper_task_override(&mut self, task: Option<Task>)`.

### 2. `src/main.rs`

- Add CLI flag:
  - `--whisper-task <transcribe|translate>`
- Merge only if set:
  - `config.apply_whisper_task_override(args.whisper_task);`
- Add startup log:
  - `Whisper task: transcribe` or `Whisper task: translate`.

### 3. `src/transcriber.rs`

- Store task in `Transcriber`.
- Explicitly set task on every inference:
  - `params.set_translate(matches!(self.task, Task::Translate));`

This removes ambiguity and guarantees default transcribe behavior.

### 4. Tests

Add/extend tests to cover task behavior end-to-end in config merge and serialization:

- `src/config.rs`:
  - missing `whisper_task` defaults to `Transcribe`
  - `"translate"` deserializes to `Task::Translate`
  - invalid/case-mismatched values fail deserialization
  - roundtrip includes both enum variants
  - override helper behavior (`None` keeps existing, `Some` replaces)
- `tests/cli_config_merge.rs`:
  - no CLI task keeps config task
  - CLI task overrides config task
- `tests/config_integration.rs` snapshot:
  - include `whisper_task` field to detect serialization regressions.

## Backward Compatibility

Existing `config.json` files without `whisper_task` will continue to load and will default to `transcribe`. No migration step is required.

## Example UX

```bash
# Default behavior (transcribe spoken language)
whisper-type

# Explicit translation mode
whisper-type --whisper-task translate
```

```json
{
  "language": "de",
  "whisper_task": "transcribe"
}
```

## Risks And Mitigations

- Risk: invalid CLI task values.
  - Mitigation: use clap value parsing with enum to fail fast and print valid options.
- Risk: config typo causes confusing behavior.
  - Mitigation: strict serde enum parsing; reject invalid values with clear startup error.

## Acceptance Criteria

1. Running without `--whisper-task` transcribes (no translation).
2. Setting `whisper_task = "translate"` in config enables translation.
3. Passing `--whisper-task translate` overrides config for that run.
4. Tests fail if default task stops being `Transcribe`.
