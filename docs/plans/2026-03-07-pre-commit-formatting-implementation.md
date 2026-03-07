# Pre-commit Formatting + VSCode DX Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add automatic `cargo fmt` on pre-commit (via cargo-husky) and VSCode format-on-save config so formatting is enforced locally before code ever reaches CI.

**Architecture:** `cargo-husky` with `user-hooks` feature reads `tests/test_hooks/pre-commit` and installs it into `.git/hooks/` on `cargo test`. VSCode settings and extension recommendations are committed to `.vscode/`. A `rustfmt.toml` documents the formatter config.

**Tech Stack:** Rust / cargo, cargo-husky v1, rustfmt, rust-analyzer (VSCode extension)

---

### Task 1: Add cargo-husky dev-dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add the dependency**

In `Cargo.toml`, find the `[dev-dependencies]` section and add:

```toml
cargo-husky = { version = "1", default-features = false, features = ["user-hooks"] }
```

The `user-hooks` feature tells cargo-husky to install whatever scripts exist in `tests/test_hooks/` rather than its built-in defaults.

**Step 2: Verify it resolves**

Run: `cargo fetch`
Expected: exits 0, no errors. (Does not require building the full project.)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): add cargo-husky for pre-commit hook management"
```

---

### Task 2: Create the pre-commit hook script

**Files:**
- Create: `tests/test_hooks/pre-commit`

**Step 1: Create the directory and script**

Create `tests/test_hooks/pre-commit` with this exact content:

```sh
#!/bin/sh
set -e
cargo fmt
git add -u
```

- `set -e` aborts on any error.
- `cargo fmt` reformats all tracked Rust source files in-place.
- `git add -u` re-stages modified tracked files so the formatter's changes are included in the commit. It does NOT add untracked files.

**Step 2: Make it executable**

```bash
chmod +x tests/test_hooks/pre-commit
```

cargo-husky copies the executable bit when installing the hook. If this is missing, the hook will be installed but silently skipped by git.

**Step 3: Verify cargo-husky installs it**

```bash
cargo test 2>&1 | head -20
ls -la .git/hooks/pre-commit
```

Expected: `.git/hooks/pre-commit` exists and is executable (`-rwxr-xr-x`).

**Step 4: Verify the hook runs on commit**

```bash
# Introduce a trivial formatting issue to test
# (e.g., add an extra blank line to src/main.rs, save, stage it)
git add src/main.rs
git commit -m "test: verify pre-commit hook"
# Expected: hook runs cargo fmt, re-stages, commit succeeds with formatted code
# Then revert the test change if you made one
```

**Step 5: Commit**

```bash
git add tests/test_hooks/pre-commit
git commit -m "chore: add pre-commit hook to auto-format with cargo fmt"
```

---

### Task 3: Add rustfmt.toml

**Files:**
- Create: `rustfmt.toml`

**Step 1: Create the file**

Create `rustfmt.toml` in the repo root:

```toml
edition = "2021"
```

This is the only non-default value that matters — it matches `edition = "2021"` in `Cargo.toml`. The file serves as a discoverable anchor for future formatting customizations.

**Step 2: Verify cargo fmt still works**

```bash
cargo fmt --check
```

Expected: exits 0 (no changes needed).

**Step 3: Commit**

```bash
git add rustfmt.toml
git commit -m "chore: add rustfmt.toml with explicit edition = 2021"
```

---

### Task 4: Add VSCode configuration

**Files:**
- Create: `.vscode/settings.json`
- Create: `.vscode/extensions.json`

**Step 1: Create `.vscode/settings.json`**

```json
{
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

This enables format-on-save for Rust files and pins the formatter to rust-analyzer (which calls `cargo fmt` under the hood).

**Step 2: Create `.vscode/extensions.json`**

```json
{
  "recommendations": ["rust-lang.rust-analyzer"]
}
```

VSCode surfaces this as a notification when the repo is first opened, prompting the developer to install rust-analyzer.

**Step 3: Verify**

Open the repo in VSCode. You should see a notification: *"This repository recommends the 'rust-lang.rust-analyzer' extension."* Editing a `.rs` file and saving should trigger formatting.

**Step 4: Commit**

```bash
git add .vscode/settings.json .vscode/extensions.json
git commit -m "chore: add VSCode settings for format-on-save and extension recommendations"
```

---

### Task 5: Update README

**Files:**
- Modify: `README.md`

**Step 1: Update the "Linting & formatting" section**

Find the existing section (around line 357):

```markdown
### Linting & formatting

```bash
cargo fmt                  # Format code
cargo fmt --check          # Check only (CI)
cargo clippy -- -D warnings  # Lint (treat warnings as errors)
```
```

Replace it with:

```markdown
### Linting & formatting

```bash
cargo fmt                    # Format code
cargo fmt --check            # Check only (CI)
cargo clippy -- -D warnings  # Lint (treat warnings as errors)
```

#### Pre-commit hook

A pre-commit hook runs `cargo fmt` automatically before every commit. It is installed by cargo-husky the first time you run:

```bash
cargo test
```

No manual setup needed. After installation, any commit will auto-format staged Rust files and re-stage them before the commit lands.

#### VSCode

Open the repo in VSCode and install the recommended extension (`rust-lang.rust-analyzer`) when prompted. This enables format-on-save, so code is typically already formatted before the pre-commit hook runs.
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: document pre-commit hook and VSCode setup in README"
```

---

### Task 6: Final verification

**Step 1: Clean-install simulation**

```bash
# Verify hook is present and executable
ls -la .git/hooks/pre-commit

# Verify formatting check passes
cargo fmt --check

# Verify tests still pass
cargo test
```

Expected: all green, hook file present and executable.

**Step 2: Verify CI compatibility**

The CI `fmt` job runs `cargo fmt --check`. Since the hook now auto-formats before every commit, this should never fail for a developer using this workflow. No CI changes are needed.
