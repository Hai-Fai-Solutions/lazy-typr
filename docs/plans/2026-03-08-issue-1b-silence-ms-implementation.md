# QA ISSUE-1B (`silence_ms`) — Implementation Plan

**Goal:** Prevent CLI default parsing from always overwriting persisted `silence_threshold_ms`; only override when `--silence-ms` is explicitly provided.

**Source QA Finding:** `docs/qa-reviews/2026-03-08-qa-finding.md` (`ISSUE-1B`)

**Scope:** Only `ISSUE-1B` (`silence_ms` merge precedence).

**Out of Scope:**
- `whisper_task` behavior and tests (`ISSUE-2` to `ISSUE-6`)
- any README/doc cleanup not required to describe `--silence-ms`

**Tech Stack:** Rust, clap 4, cargo test

---

### Task 1: Make `--silence-ms` explicit-only at CLI layer

**Files:**
- Modify: `src/main.rs`

**Step 1: Change `Args` field type**

Update CLI args struct from:

```rust
#[arg(long, default_value = "800")]
silence_ms: u64,
```

to:

```rust
#[arg(long)]
silence_ms: Option<u64>,
```

**Step 2: Preserve help text intent**

Keep description aligned with effective default source:
- default value comes from config file when present
- otherwise from `Config::default()` (`800`)

---

### Task 2: Apply silence threshold override conditionally

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`

**Step 1: Add config helper for consistency**

In `Config` impl, add:

```rust
pub fn apply_silence_override(&mut self, silence_ms: Option<u64>) {
    if let Some(silence_ms) = silence_ms {
        self.silence_threshold_ms = silence_ms;
    }
}
```

This mirrors existing `apply_language_override` semantics.

**Step 2: Use helper in merge block**

Replace unconditional assignment in `main()`:

```rust
config.silence_threshold_ms = args.silence_ms;
```

with:

```rust
config.apply_silence_override(args.silence_ms);
```

**Step 3: Optional validation (defer unless required)**

If desired, enforce `--silence-ms >= 1` via clap value parser. Keep out of this change if scope must remain strictly precedence-only.

---

### Task 3: Add regression tests for silence merge precedence

**Files:**
- Modify: `tests/cli_config_merge.rs`
- Modify: `src/config.rs` test module

**Step 1: Unit tests in `Config`**

Add tests:
- `test_apply_silence_override_with_none_keeps_existing_value`
- `test_apply_silence_override_with_some_replaces_value`

**Step 2: Merge behavior regression tests**

Extend `tests/cli_config_merge.rs` with:
- `cli_without_silence_ms_does_not_mutate_silence_threshold`
- `cli_silence_ms_overrides_config`

Ensure existing `no_cli_flags_keeps_config_values` still asserts silence is preserved.

---

### Task 4: Verification

**Step 1: Focused tests**

```bash
cargo test cli_config_merge -- --nocapture
```

Expected: silence precedence tests pass.

**Step 2: Config tests**

```bash
cargo test config::tests -- --nocapture
```

Expected: new `apply_silence_override` unit tests pass.

**Step 3: Full suite**

```bash
cargo test
```

Expected: no regressions.

---

### Task 5: Manual runtime checks (recommended)

1. Put `"silence_threshold_ms": 1200` in config file.
2. Run app without `--silence-ms`; verify startup log shows `Silence threshold: 1200ms` (when not in PTT mode).
3. Run app with `--silence-ms 500`; verify startup log shows `Silence threshold: 500ms`.

---

## Acceptance Criteria

1. No `--silence-ms` flag: persisted config value is preserved.
2. `--silence-ms <N>`: runtime config uses `<N>`.
3. Regression tests fail if unconditional overwrite returns.
4. Existing language merge behavior remains unchanged.

## Risks and Mitigations

1. Risk: Future CLI refactors reintroduce default-value overwrite.
- Mitigation: keep explicit regression tests in `tests/cli_config_merge.rs`.

2. Risk: Users assume CLI shows hard default from clap.
- Mitigation: use clear arg help text and rely on runtime/log output for effective value.
