# Transcribe-By-Default Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `Task` enum to `Config`, expose `--whisper-task` on the CLI, store the task in `Transcriber`, and set it explicitly on every Whisper inference so default behavior is always `transcribe`.

**Architecture:** Four small changes across four files — `config.rs` (enum + field + helper), `main.rs` (CLI flag + merge + log), `transcriber.rs` (store + apply task on inference), and two integration test files. No new files beyond the plan. Strict serde parsing rejects typos at startup.

**Tech Stack:** Rust, `serde_json`, `clap` (ValueEnum), `whisper-rs` (FullParams::set_translate)

---

### Task 1: Add `Task` enum and `whisper_task` field to `src/config.rs`

**Files:**
- Modify: `src/config.rs`

**Step 1: Write the failing unit tests**

Add these tests inside the existing `#[cfg(test)] mod tests` block in `src/config.rs`, after the last test:

```rust
// ── Task enum ─────────────────────────────────────────────────────────────

#[test]
fn test_default_whisper_task_is_transcribe() {
    assert_eq!(Config::default().whisper_task, Task::Transcribe);
}

#[test]
fn test_task_translate_deserializes_from_json() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01,
        "whisper_task": "translate"
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.whisper_task, Task::Translate);
}

#[test]
fn test_task_transcribe_deserializes_from_json() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01,
        "whisper_task": "transcribe"
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.whisper_task, Task::Transcribe);
}

#[test]
fn test_task_invalid_value_fails_deserialization() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01,
        "whisper_task": "Translate"
    }"#;
    let result: Result<Config, _> = serde_json::from_str(json);
    assert!(result.is_err(), "uppercase 'Translate' must be rejected");
}

#[test]
fn test_task_absent_in_legacy_json_defaults_to_transcribe() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.whisper_task, Task::Transcribe);
}

#[test]
fn test_task_roundtrips_transcribe() {
    let cfg = Config {
        whisper_task: Task::Transcribe,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.whisper_task, Task::Transcribe);
}

#[test]
fn test_task_roundtrips_translate() {
    let cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.whisper_task, Task::Translate);
}

#[test]
fn test_apply_whisper_task_override_with_none_keeps_existing() {
    let mut cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    cfg.apply_whisper_task_override(None);
    assert_eq!(cfg.whisper_task, Task::Translate);
}

#[test]
fn test_apply_whisper_task_override_with_some_replaces_value() {
    let mut cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    cfg.apply_whisper_task_override(Some(Task::Transcribe));
    assert_eq!(cfg.whisper_task, Task::Transcribe);
}
```

**Step 2: Run to confirm the tests fail to compile**

```bash
cargo test -q 2>&1 | head -20
```

Expected: compile error — `Task` not found, `whisper_task` field not found.

**Step 3: Add the `Task` enum**

Add this block immediately before the `Config` struct definition (around line 5):

```rust
/// Whisper inference task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    Transcribe,
    Translate,
}

impl Default for Task {
    fn default() -> Self {
        Task::Transcribe
    }
}
```

**Step 4: Add `whisper_task` field to `Config`**

In the `Config` struct, add after the `gpu_device` field (around line 42):

```rust
    /// Whisper inference task: transcribe (default) or translate.
    #[serde(default)]
    pub whisper_task: Task,
```

**Step 5: Add `whisper_task` to `Config::default()`**

In the `Default` impl body, add after `gpu_device: 0,`:

```rust
            whisper_task: Task::default(),
```

**Step 6: Add `apply_whisper_task_override` helper**

In the `impl Config` block, add after `apply_language_override`:

```rust
    /// Apply CLI whisper_task override only when it is explicitly provided.
    pub fn apply_whisper_task_override(&mut self, task: Option<Task>) {
        if let Some(task) = task {
            self.whisper_task = task;
        }
    }
```

**Step 7: Run tests**

```bash
cargo test -q
```

Expected: all tests pass, no warnings.

**Step 8: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add Task enum and whisper_task field with serde default"
```

---

### Task 2: Add `--whisper-task` CLI flag to `src/main.rs`

**Files:**
- Modify: `src/main.rs`

**Step 1: Import `Task`**

At the top of `src/main.rs`, update the config import line from:

```rust
use whisper_type::config::Config;
```

to:

```rust
use whisper_type::config::{Config, Task};
```

**Step 2: Add the CLI argument to `Args`**

In the `Args` struct, add after the `webrtc_vad_aggressiveness` field (around line 58):

```rust
    /// Whisper inference task: transcribe (default) or translate
    #[arg(long, value_enum)]
    whisper_task: Option<Task>,
```

**Step 3: Merge the CLI value into config**

In `main()`, after the `apply_language_override` call (around line 105):

```rust
    config.apply_whisper_task_override(args.whisper_task);
```

**Step 4: Add startup log**

In the startup info block (after the `Language:` log, around line 162):

```rust
    info!(
        "Whisper task: {}",
        match config.whisper_task {
            Task::Transcribe => "transcribe",
            Task::Translate => "translate",
        }
    );
```

**Step 5: Build to confirm it compiles**

```bash
cargo build 2>&1 | grep -E "^error"
```

Expected: no output (no errors).

**Step 6: Smoke-test CLI help shows the new flag**

```bash
cargo run -- --help 2>&1 | grep whisper-task
```

Expected: `--whisper-task <WHISPER_TASK>  Whisper inference task: transcribe (default) or translate`

**Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): add --whisper-task flag with transcribe/translate values"
```

---

### Task 3: Store task in `Transcriber` and apply on every inference

**Files:**
- Modify: `src/transcriber.rs`

**Step 1: Import `Task`**

Update the existing config import at the top of `src/transcriber.rs` from:

```rust
use crate::config::Config;
```

to:

```rust
use crate::config::{Config, Task};
```

**Step 2: Add `task` field to `Transcriber`**

Update the struct definition from:

```rust
pub struct Transcriber {
    ctx: WhisperContext,
    language: String,
}
```

to:

```rust
pub struct Transcriber {
    ctx: WhisperContext,
    language: String,
    task: Task,
}
```

**Step 3: Initialise `task` in `Transcriber::new`**

Update the `Ok(Self { ... })` block to include `task`:

```rust
        Ok(Self {
            ctx,
            language: config.language.clone(),
            task: config.whisper_task.clone(),
        })
```

**Step 4: Set translate flag on every inference**

In `transcribe()`, after the language block (around line 43), add:

```rust
        // Task: explicitly set on every inference to prevent silent drift
        params.set_translate(matches!(self.task, Task::Translate));
```

**Step 5: Build and run unit tests**

```bash
cargo test -q
```

Expected: all tests pass, no warnings.

**Step 6: Commit**

```bash
git add src/transcriber.rs
git commit -m "feat(transcriber): store task and set translate flag on every inference"
```

---

### Task 4: Integration tests

**Files:**
- Modify: `tests/cli_config_merge.rs`
- Modify: `tests/config_integration.rs`

**Step 1: Add task override tests to `tests/cli_config_merge.rs`**

Update the import at the top from:

```rust
use whisper_type::config::Config;
```

to:

```rust
use whisper_type::config::{Config, Task};
```

Then add these two tests at the end of the file:

```rust
#[test]
fn no_cli_task_keeps_config_task() {
    let mut cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    cfg.apply_whisper_task_override(None);
    assert_eq!(cfg.whisper_task, Task::Translate);
}

#[test]
fn cli_task_overrides_config_task() {
    let mut cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    cfg.apply_whisper_task_override(Some(Task::Transcribe));
    assert_eq!(cfg.whisper_task, Task::Transcribe);
}
```

**Step 2: Add `whisper_task` to the snapshot in `tests/config_integration.rs`**

In the `ConfigSnapshot` struct, add after `ptt_key`:

```rust
    #[serde(default = "default_whisper_task")]
    whisper_task: String,
```

Add the default function after `default_log_level`:

```rust
fn default_whisper_task() -> String {
    "transcribe".to_string()
}
```

Update `config_full_roundtrip` to include the field in `original`:

```rust
        whisper_task: "translate".to_string(),
```

Update `config_missing_optional_fields_use_defaults` to assert the default:

```rust
    assert_eq!(cfg.whisper_task, "transcribe");
```

**Step 3: Run all tests**

```bash
cargo test 2>&1 | tail -5
```

Expected: `test result: ok. N passed; 0 failed`

**Step 4: Commit**

```bash
git add tests/cli_config_merge.rs tests/config_integration.rs
git commit -m "test: add whisper_task CLI merge and config snapshot regression tests"
```

---

## Acceptance Criteria Verification

After all tasks, run:

```bash
cargo test -q
```

Manually verify:
1. `cargo run -- --dry-run` logs `Whisper task: transcribe`
2. `cargo run -- --whisper-task translate --dry-run` logs `Whisper task: translate`
3. `cargo run -- --whisper-task Translate --dry-run` prints a clap error and exits non-zero
