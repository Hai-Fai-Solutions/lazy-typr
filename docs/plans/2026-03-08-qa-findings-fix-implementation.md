# QA Findings Language Fix — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the QA fix so persisted config language is not overwritten unless `--language` is explicitly passed.

**Source Design:** `docs/plans/2026-03-08-qa-findings-fix-design.md`

**Scope Decision:** Only the language override fix was selected. No implementation work for `silence_ms`, `whisper_task`, or other QA findings in this plan.

**Tech Stack:** Rust, clap 4, serde, cargo test

---

### Task 1: Change CLI argument semantics for language

**Files:**
- Modify: `src/main.rs`

**Step 1: Make `language` optional in `Args`**

In `src/main.rs`, update:

```rust
#[arg(short, long, default_value = "de")]
language: String,
```

to:

```rust
#[arg(short, long)]
language: Option<String>,
```

**Step 2: Apply language override conditionally**

In the CLI-to-config merge block in `main()`, replace:

```rust
config.language = args.language;
```

with:

```rust
if let Some(language) = args.language {
    config.language = language;
}
```

**Step 3: Compile check**

Run:

```bash
cargo build
```

Expected: exits 0.

---

### Task 2: Add regression tests for language merge precedence

**Files:**
- Modify: `src/main.rs` (extract helper if needed)
- Add: `tests/cli_config_merge.rs`

**Step 1: Make merge logic testable**

If needed, extract CLI override logic into a helper (example):

```rust
fn apply_cli_overrides(config: &mut Config, args: &Args)
```

Keep runtime behavior unchanged.

**Step 2: Add tests covering only language precedence**

Create `tests/cli_config_merge.rs` with tests:

- `no_cli_flags_keeps_config_values`
- `cli_language_overrides_config`
- `cli_without_language_does_not_mutate_language`

**Step 3: Run focused tests**

Run:

```bash
cargo test cli_config_merge -- --nocapture
```

Expected: all new tests pass.

---

### Task 3: Update README wording for language default behavior

**Files:**
- Modify: `README.md`

**Step 1: Adjust CLI usage description**

Ensure README does not imply clap-level forced default for `--language`.

**Step 2: Clarify precedence**

Document that effective language comes from:

1. `--language` (if provided)
2. config file value
3. built-in default (`de`) when no config exists

**Step 3: Verify docs consistency**

Check README examples and option table for conflicting wording.

---

### Task 4: Full verification

**Step 1: Run all tests**

```bash
cargo test
```

Expected: full suite passes.

**Step 2: Manual runtime check (optional but recommended)**

- Set config language to `fr`.
- Run `whisper-type` with no `--language`.
- Confirm startup log shows `Language: fr`.
- Run `whisper-type --language en`.
- Confirm startup log shows `Language: en`.

---

### Task 5: Commit

```bash
git add src/main.rs tests/cli_config_merge.rs README.md
git commit -m "fix: preserve config language unless --language is provided"
```

---

## Acceptance Criteria

1. Running without `--language` preserves persisted config language.
2. `--language` overrides persisted language for the run.
3. Regression tests cover and enforce language precedence behavior.
4. README accurately describes language precedence.

## Out of Scope (Explicit)

- `silence_ms` CLI/config merge changes
- `whisper_task` parsing/serde/test coverage
- Any additional QA findings not related to language override

## Review Findings (Post-Implementation)

1. Low: Public API compatibility risk  
Status: `Open`  
Detail: Removing `src/cli_overrides.rs` and `pub mod cli_overrides` from `src/lib.rs` can break external consumers importing `whisper_type::cli_overrides`.

2. Low: Reduced end-to-end CLI merge regression coverage  
Status: `Open`  
Detail: `tests/cli_config_merge.rs` now validates `Config::apply_language_override` directly, but does not exercise clap parsing/wiring in `main.rs`.

## Follow-up Actions for Findings

1. Decide API compatibility intent:
- If preserving compatibility, re-export a deprecated shim module for one release cycle.
- If not preserving compatibility, call out the breaking change in release notes.

2. Add one CLI-path regression test:
- Add a parsing-level test that verifies absence/presence of `--language` results in expected effective config language when merged.
