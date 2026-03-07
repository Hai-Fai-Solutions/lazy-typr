# Pre-commit Formatting + VSCode DX — Design

**Date:** 2026-03-07
**Status:** Approved

## Problem

Developers can commit unformatted Rust code. CI catches it with `cargo fmt --check`, but that means a wasted pipeline run and a fix commit. The goal is to eliminate the feedback gap by formatting automatically at commit time and in the editor.

## Decisions

| Question | Answer |
|----------|--------|
| Hook behaviour | Auto-format (`cargo fmt` + re-stage), not fail-fast |
| Hook distribution | `cargo-husky` — installs on `cargo test`, zero manual setup |
| VSCode integration | Format-on-save via rust-analyzer; shared via committed `.vscode/` |
| Formatter config | `rustfmt.toml` with explicit `edition = "2021"` |

## Components

### 1. `cargo-husky` dev-dependency (`Cargo.toml`)

```toml
cargo-husky = { version = "1", default-features = false, features = ["user-hooks"] }
```

With `user-hooks`, cargo-husky installs scripts from `tests/test_hooks/` into `.git/hooks/` the next time `cargo test` runs. No manual `git config` step needed.

### 2. `tests/test_hooks/pre-commit`

```sh
#!/bin/sh
set -e
cargo fmt
git add -u
```

- `cargo fmt` reformats all tracked Rust source files.
- `git add -u` re-stages any files modified by the formatter (only already-tracked files — no accidental new additions).
- The commit then proceeds with clean, formatted code.
- Must be executable (`chmod +x`).

### 3. `.vscode/settings.json`

```json
{
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

Formats on every save so the hook is rarely triggered in practice.

### 4. `.vscode/extensions.json`

```json
{
  "recommendations": ["rust-lang.rust-analyzer"]
}
```

VSCode surfaces this when the repo is first opened, prompting installation of rust-analyzer.

### 5. `rustfmt.toml`

```toml
edition = "2021"
```

Explicitly documents the Rust edition used for formatting. Matches `Cargo.toml`. Acts as a discoverable anchor for future formatting customizations.

## Developer experience (end-to-end)

1. Clone repo → open in VSCode → prompted to install rust-analyzer
2. `cargo test` (once) → hook installed automatically
3. Edit Rust → VSCode formats on save
4. `git commit` → hook runs `cargo fmt` + re-stages as safety net → commit proceeds

CI's `cargo fmt --check` remains the final enforcement layer.

## README changes

Add a note to the "Linting & formatting" section explaining:
- `cargo test` installs the pre-commit hook automatically
- VSCode setup: open repo, install recommended extensions
