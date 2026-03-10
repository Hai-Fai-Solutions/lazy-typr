# Translate Flag Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the `Task` enum and `--whisper-task` CLI arg with a simple `translate: bool` field and `--translate` flag.

**Architecture:** Remove the `Task` enum entirely. `Config.whisper_task` becomes `Config.translate: bool`. The CLI gains a bare `--translate` flag (presence = true). `Transcriber` uses the bool directly. Breaking change — no migration path.

**Tech Stack:** Rust, clap 4 (derive), serde_json, whisper-rs

---

### Task 1: Replace `Config.whisper_task` with `Config.translate` in `config.rs`

**Files:**
- Modify: `src/config.rs`

#### Step 1: Write failing tests

In `src/config.rs`, inside the `#[cfg(test)] mod tests` block, **replace** the entire `// ── Task enum ─────────────────────` section (lines 423–549) with these tests:

```rust
// ── translate flag ────────────────────────────────────────────────────────

#[test]
fn test_default_translate_is_false() {
    assert!(!Config::default().translate);
}

#[test]
fn test_translate_true_deserializes_from_json() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01,
        "translate": true
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert!(cfg.translate);
}

#[test]
fn test_translate_false_deserializes_from_json() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01,
        "translate": false
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert!(!cfg.translate);
}

#[test]
fn test_translate_absent_in_legacy_json_defaults_to_false() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;
    let cfg: Config = serde_json::from_str(json).unwrap();
    assert!(!cfg.translate);
}

#[test]
fn test_translate_roundtrips_true() {
    let cfg = Config {
        translate: true,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert!(restored.translate);
}

#[test]
fn test_translate_roundtrips_false() {
    let cfg = Config {
        translate: false,
        ..Config::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();
    assert!(!restored.translate);
}

#[test]
fn test_apply_translate_override_false_keeps_existing() {
    let mut cfg = Config {
        translate: true,
        ..Config::default()
    };
    cfg.apply_translate_override(false);
    assert!(cfg.translate);
}

#[test]
fn test_apply_translate_override_true_sets_translate() {
    let mut cfg = Config::default(); // translate = false
    cfg.apply_translate_override(true);
    assert!(cfg.translate);
}
```

#### Step 2: Run tests to verify they fail

```bash
cargo test --lib translate 2>&1 | head -40
```

Expected: compile error — `translate` field and `apply_translate_override` do not exist yet.

#### Step 3: Implement the changes in `config.rs`

**3a.** Delete lines 7–14 (the entire `Task` enum):
```rust
// DELETE this block:
/// Whisper inference task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, clap::ValueEnum, Default)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    #[default]
    Transcribe,
    Translate,
}
```

**3b.** Replace lines 56–58:
```rust
// OLD:
/// Whisper inference task: transcribe (default) or translate.
#[serde(default)]
pub whisper_task: Task,

// NEW:
/// Translate speech to English instead of transcribing in the source language.
#[serde(default)]
pub translate: bool,
```

**3c.** In `impl Default for Config`, replace line 97:
```rust
// OLD:
whisper_task: Task::default(),

// NEW:
translate: false,
```

**3d.** Replace the `apply_whisper_task_override` method (lines 160–165):
```rust
// OLD:
/// Apply CLI whisper_task override only when it is explicitly provided.
pub fn apply_whisper_task_override(&mut self, task: Option<Task>) {
    if let Some(task) = task {
        self.whisper_task = task;
    }
}

// NEW:
/// Apply CLI translate override only when the flag is explicitly passed.
pub fn apply_translate_override(&mut self, translate: bool) {
    if translate {
        self.translate = true;
    }
}
```

#### Step 4: Run tests to verify they pass

```bash
cargo test --lib translate 2>&1
```

Expected: all 8 new tests pass; any remaining `Task`-related tests now fail to compile (they'll be cleaned up in this step too — they were already removed in Step 1).

#### Step 5: Verify full lib test suite compiles (transcriber and main will still fail — that's expected)

```bash
cargo test --lib 2>&1 | tail -20
```

Expected: lib tests pass; binary (`main.rs`) fails to compile because it still references `Task`. That's fine for now.

#### Step 6: Commit

```bash
git add src/config.rs
git commit -m "refactor(config): replace Task enum with translate: bool"
```

---

### Task 2: Update `src/transcriber.rs`

**Files:**
- Modify: `src/transcriber.rs`

#### Step 1: Update the import

```rust
// OLD (line 5):
use crate::config::{Config, Task};

// NEW:
use crate::config::Config;
```

#### Step 2: Replace the `task: Task` field with `translate: bool`

```rust
// OLD (lines 26–30):
pub struct Transcriber {
    ctx: WhisperContext,
    language: String,
    task: Task,
}

// NEW:
pub struct Transcriber {
    ctx: WhisperContext,
    language: String,
    translate: bool,
}
```

#### Step 3: Update the constructor

```rust
// OLD (lines 40–44):
Ok(Self {
    ctx,
    language: config.language.clone(),
    task: config.whisper_task.clone(),
})

// NEW:
Ok(Self {
    ctx,
    language: config.language.clone(),
    translate: config.translate,
})
```

#### Step 4: Update the inference call

```rust
// OLD (line 64):
params.set_translate(matches!(self.task, Task::Translate));

// NEW:
params.set_translate(self.translate);
```

#### Step 5: Run transcriber tests

```bash
cargo test --lib transcriber 2>&1
```

Expected: all tests pass.

#### Step 6: Commit

```bash
git add src/transcriber.rs
git commit -m "refactor(transcriber): use translate: bool instead of Task enum"
```

---

### Task 3: Update `src/main.rs`

**Files:**
- Modify: `src/main.rs`

#### Step 1: Update the import

```rust
// OLD (line 11):
use whisper_type::config::{Config, Task};

// NEW:
use whisper_type::config::Config;
```

#### Step 2: Replace `--whisper-task` arg with `--translate`

```rust
// OLD (lines 64–66):
/// Whisper inference task: transcribe (default) or translate
#[arg(long, value_enum)]
whisper_task: Option<Task>,

// NEW:
/// Translate speech to English instead of transcribing in the source language
#[arg(long)]
translate: bool,
```

#### Step 3: Replace the override call

```rust
// OLD (line 100):
config.apply_whisper_task_override(args.whisper_task);

// NEW:
config.apply_translate_override(args.translate);
```

#### Step 4: Replace the startup log

```rust
// OLD (lines 171–177):
info!(
    "Whisper task: {}",
    match config.whisper_task {
        Task::Transcribe => "transcribe",
        Task::Translate => "translate",
    }
);

// NEW:
info!("Translate: {}", config.translate);
```

#### Step 5: Build to verify everything compiles

```bash
cargo build 2>&1
```

Expected: clean build, zero errors.

#### Step 6: Run the full test suite

```bash
cargo test 2>&1
```

Expected: all tests pass.

#### Step 7: Commit

```bash
git add src/main.rs
git commit -m "feat: replace --whisper-task with --translate boolean flag"
```

---

### Task 4: Update `tests/cli_config_merge.rs`

**Files:**
- Modify: `tests/cli_config_merge.rs`

#### Step 1: Write failing tests

Add these two tests to the end of the file:

```rust
#[test]
fn no_translate_flag_keeps_config_translate() {
    let mut cfg = Config {
        translate: true,
        ..Config::default()
    };
    cfg.apply_translate_override(false);
    assert!(cfg.translate);
}

#[test]
fn translate_flag_overrides_config_translate() {
    let mut cfg = Config::default(); // translate = false
    cfg.apply_translate_override(true);
    assert!(cfg.translate);
}
```

#### Step 2: Run to verify new tests pass (they should, as implementation is already done)

```bash
cargo test --test cli_config_merge 2>&1
```

Expected: all tests pass.

#### Step 3: Remove the old `Task`-based tests

Delete these two functions from `tests/cli_config_merge.rs`:

```rust
// DELETE:
#[test]
fn no_cli_task_keeps_config_task() { ... }

// DELETE:
#[test]
fn cli_task_overrides_config_task() { ... }
```

Also remove the `Task` from the import on line 1:

```rust
// OLD:
use whisper_type::config::{Config, Task};

// NEW:
use whisper_type::config::Config;
```

#### Step 4: Run full test suite

```bash
cargo test 2>&1
```

Expected: all tests pass, zero warnings about unused imports.

#### Step 5: Lint check

```bash
cargo clippy -- -D warnings 2>&1
```

Expected: clean.

#### Step 6: Commit

```bash
git add tests/cli_config_merge.rs
git commit -m "test: replace Task-based CLI merge tests with translate bool tests"
```

---

### Task 5: Update QA findings doc

**Files:**
- Modify: `docs/qa-reviews/2026-03-08-qa-finding.md`

#### Step 1: Mark ISSUE-2 as Solved

Change:

```markdown
3. `ISSUE-2`
Title: Silent fallback on `--whisper-task` typo
Status: `Open`
Details: `--whisper-task trnascribe` silently falls back without warning.
```

To:

```markdown
3. `ISSUE-2`
Title: Silent fallback on `--whisper-task` typo
Status: `Solved`
Details: `--whisper-task` removed entirely. Replaced with `--translate` boolean flag — no string to mistype.
```

#### Step 2: Commit

```bash
git add docs/qa-reviews/2026-03-08-qa-finding.md
git commit -m "docs: mark ISSUE-2 as solved"
```
