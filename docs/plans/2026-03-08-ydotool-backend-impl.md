# ydotool Backend Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `Backend::Ydotool` typing backend to `src/typer.rs` so KDE Plasma (and any compositor that lacks `zwlr_virtual_keyboard_v1`) gets reliable text injection; update `setup.sh` and `README.md` to document it.

**Architecture:** A third enum variant `Backend::Ydotool` is probed first (before checking `WAYLAND_DISPLAY`) so ydotool silently wins on any compositor when installed; users without ydotool are completely unaffected. The new typing functions mirror the existing wtype/xdotool functions exactly — same save/type/restore clipboard pattern, no new crates.

**Tech Stack:** Rust, `std::process::Command`, `ydotool` CLI, `ydotoold` user daemon, Bash (`setup.sh`)

---

### Task 1: Add `Backend::Ydotool` enum variant and make the project compile

All existing `match backend` arms are exhaustive — adding the variant without handling it is a compile error. This task adds the variant and stubs every new arm with `unreachable!()` so we get a clean compile immediately, then adds one smoke-test to confirm the variant exists and dry-run still works.

**Files:**
- Modify: `src/typer.rs:8-11` (enum definition)
- Modify: `src/typer.rs:22-33` (`probe_tool` match)
- Modify: `src/typer.rs:48-57` (tool-unavailable warning match)
- Modify: `src/typer.rs:83-102` (`type_text` match)

**Step 1: Add the variant to the enum**

In `src/typer.rs`, change the `Backend` enum (currently at line 8) from:
```rust
enum Backend {
    X11,
    Wayland,
}
```
to:
```rust
enum Backend {
    X11,
    Wayland,
    Ydotool,
}
```

**Step 2: Stub every exhaustive match arm**

In `probe_tool` (match at line 23), add the arm before the closing brace:
```rust
Backend::Ydotool => ("ydotool", "--help"),
```

In the tool-unavailable warning match inside `Typer::new` (line 49–51), add:
```rust
Backend::Ydotool => "ydotool",
```

In `type_text` (match at line 83), add a temporary arm after the `Backend::Wayland` arm:
```rust
Backend::Ydotool => {
    unreachable!("ydotool path not yet implemented")
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles with zero errors.

**Step 4: Write a smoke test for the new variant**

Add this test at the bottom of the `#[cfg(test)]` block in `src/typer.rs`:
```rust
/// Smoke test: Backend::Ydotool variant exists and dry-run path does not reach the unreachable arm.
#[test]
fn test_dry_run_with_ydotool_backend() {
    let typer = Typer {
        dry_run: true,
        backend: Backend::Ydotool,
        tool_available: true,
    };
    assert!(typer.type_text("hello kde").is_ok());
}
```

**Step 5: Run the test**

Run: `cargo test test_dry_run_with_ydotool_backend -- --nocapture`
Expected: `test test_dry_run_with_ydotool_backend ... ok`

**Step 6: Run the full suite**

Run: `cargo test`
Expected: all 78 tests pass.

**Step 7: Commit**

```bash
git add src/typer.rs
git commit -m "feat: add Backend::Ydotool skeleton (stubs, dry-run test)"
```

---

### Task 2: Implement `type_with_ydotool`

Mirror `type_with_wtype` exactly — same `output()` pattern, same error handling.

**Files:**
- Modify: `src/typer.rs` (add new method after `type_with_wtype`)

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` block:
```rust
/// When tool_available=false, type_text with Ydotool backend must skip direct typing.
/// (The clipboard path may fail in CI — that is acceptable; we only check it does not panic
/// and does not attempt the direct ydotool call.)
#[test]
fn test_type_text_skips_direct_typing_when_unavailable_ydotool() {
    let typer = Typer {
        dry_run: false,
        backend: Backend::Ydotool,
        tool_available: false,
    };
    // Must not panic. Result may be Err (no clipboard in CI).
    let _ = typer.type_text("test");
}
```

Run: `cargo test test_type_text_skips_direct_typing_when_unavailable_ydotool`
Expected: FAIL — compilation error because `Backend::Ydotool` arm is `unreachable!()`, which panics at runtime, causing the test to fail.

**Step 2: Implement `type_with_ydotool`**

After the `type_with_wtype` function (around line 133), add:
```rust
fn type_with_ydotool(&self, text: &str) -> Result<()> {
    let output = std::process::Command::new("ydotool")
        .args(["type", "--key-delay=0", "--key-hold=0", "--", text])
        .output()
        .context("ydotool not found. Install with: sudo pacman -S ydotool")?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ydotool failed: {}", stderr)
    }
}
```

**Step 3: Wire it into `type_text`**

Replace the `unreachable!()` stub in the `Backend::Ydotool` arm with:
```rust
Backend::Ydotool => {
    if self.tool_available && self.type_with_ydotool(&text_with_space).is_ok() {
        return Ok(());
    }
    if self.tool_available {
        info!("ydotool failed, falling back to clipboard paste");
    }
    self.type_with_clipboard_ydotool(&text_with_space)
}
```

Note: `type_with_clipboard_ydotool` doesn't exist yet — add a temporary stub below the arm to make it compile:
```rust
fn type_with_clipboard_ydotool(&self, _text: &str) -> Result<()> {
    anyhow::bail!("not yet implemented")
}
```

**Step 4: Run the test**

Run: `cargo test test_type_text_skips_direct_typing_when_unavailable_ydotool`
Expected: passes (hits clipboard stub which returns Err, but test only checks no-panic).

**Step 5: Run the full suite**

Run: `cargo test`
Expected: all 79 tests pass (78 original + 2 new, minus the stub issue: actually 80 tests now with both new ones).

**Step 6: Commit**

```bash
git add src/typer.rs
git commit -m "feat: implement type_with_ydotool (stub clipboard fallback)"
```

---

### Task 3: Implement `type_with_clipboard_ydotool` and extend `set_clipboard`

Mirror `type_with_clipboard_wayland` — save clipboard, set clipboard, send `ydotool key ctrl+v`, restore clipboard.

**Files:**
- Modify: `src/typer.rs` — replace `type_with_clipboard_ydotool` stub, extend `set_clipboard`

**Step 1: Write a compile-time constant for the ydotool paste args**

At the top of `src/typer.rs`, alongside `WTYPE_PASTE_ARGS`, add:
```rust
/// Arguments for ydotool to send Ctrl+V.
const YDOTOOL_PASTE_ARGS: &[&str] = &["key", "ctrl+v"];
```

**Step 2: Write the test for the constant**

Add to the `#[cfg(test)]` block:
```rust
#[test]
fn test_ydotool_key_combo_args_are_correct() {
    assert_eq!(
        YDOTOOL_PASTE_ARGS,
        &["key", "ctrl+v"],
        "ydotool paste args must be 'ydotool key ctrl+v'"
    );
}
```

Run: `cargo test test_ydotool_key_combo_args_are_correct`
Expected: PASS.

**Step 3: Replace the `type_with_clipboard_ydotool` stub**

Replace the stub body with the real implementation:
```rust
fn type_with_clipboard_ydotool(&self, text: &str) -> Result<()> {
    let saved = self.get_clipboard();

    self.set_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    let paste_result = std::process::Command::new("ydotool")
        .args(YDOTOOL_PASTE_ARGS)
        .status();

    if let Ok(saved_text) = saved {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = self.set_clipboard(&saved_text);
    }

    paste_result
        .context("ydotool key failed")?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("ydotool key ctrl+v failed"))
}
```

**Step 4: Extend `set_clipboard` to use `wl-copy` for Ydotool**

In `set_clipboard`, change the condition on line 182 from:
```rust
if matches!(self.backend, Backend::Wayland) {
```
to:
```rust
if matches!(self.backend, Backend::Wayland | Backend::Ydotool) {
```

This ensures `wl-copy` is tried first when the session has a Wayland display (which is true whenever ydotool is used on Wayland compositors like KDE).

**Step 5: Run the full suite**

Run: `cargo test`
Expected: all 81 tests pass.

**Step 6: Commit**

```bash
git add src/typer.rs
git commit -m "feat: implement type_with_clipboard_ydotool, extend set_clipboard for ydotool"
```

---

### Task 4: Update `Typer::new` — probe ydotool first

Currently `Typer::new` checks `WAYLAND_DISPLAY` first. The new detection order is: ydotool probe → `WAYLAND_DISPLAY` → X11.

**Files:**
- Modify: `src/typer.rs:36-66` (`Typer::new`)

**Step 1: Understand the existing detection logic**

Currently (lines 36–43):
```rust
let backend = if std::env::var("WAYLAND_DISPLAY").is_ok() {
    info!("Wayland display detected — using wtype");
    Backend::Wayland
} else {
    Backend::X11
};
```

**Step 2: Replace with three-level detection**

Replace those lines with:
```rust
let backend = if Self::probe_tool(&Backend::Ydotool) {
    info!("ydotool detected — using ydotool (compositor-agnostic)");
    Backend::Ydotool
} else if std::env::var("WAYLAND_DISPLAY").is_ok() {
    info!("Wayland display detected — using wtype");
    Backend::Wayland
} else {
    Backend::X11
};
```

**Step 3: Run the full suite**

Run: `cargo test`
Expected: all 81 tests pass.

Note: `test_tool_available_false_when_binary_missing` clears PATH so `ydotool` probe returns false, falls through to Wayland/X11, and `tool_available` ends up false — still passes.

**Step 4: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

**Step 5: Commit**

```bash
git add src/typer.rs
git commit -m "feat: probe ydotool first in Typer::new detection order"
```

---

### Task 5: Update `setup.sh` — install ydotool on KDE Wayland

**Files:**
- Modify: `setup.sh:82-96` (Arch Wayland block) and `setup.sh:119-132` (Debian Wayland block)

**Step 1: Add ydotool install to the Arch Wayland block**

After the `wl-clipboard` block (after the `ok "wl-clipboard"` line, around line 95), still inside the `if [ "$XDG_SESSION_TYPE" = "wayland" ]` block, add:

```bash
        # ydotool: compositor-agnostic text injection (required for KDE, recommended for others)
        local desktop_lower="${XDG_CURRENT_DESKTOP,,}"
        if [[ "$desktop_lower" == *"kde"* ]]; then
            if ! command -v ydotool &>/dev/null; then
                log "KDE Plasma detected — installing ydotool (required for text injection)"
                sudo pacman -S --noconfirm ydotool
            else
                ok "ydotool (KDE Wayland)"
            fi
        else
            if ! command -v ydotool &>/dev/null; then
                warn "ydotool not found — text injection falls back to wtype; install ydotool for compositor-agnostic support"
            else
                ok "ydotool (optional, detected)"
            fi
        fi
        if command -v ydotool &>/dev/null; then
            log "Enabling ydotoold user service..."
            systemctl --user enable --now ydotoold || warn "Could not enable ydotoold — run: systemctl --user enable --now ydotoold"
        fi
```

**Step 2: Add the same block to the Debian Wayland section**

After the `ok "wl-clipboard"` / `wl-clipboard` install block inside the Debian `if [ "$XDG_SESSION_TYPE" = "wayland" ]` block, add:

```bash
        # ydotool: compositor-agnostic text injection (required for KDE, recommended for others)
        local desktop_lower="${XDG_CURRENT_DESKTOP,,}"
        if [[ "$desktop_lower" == *"kde"* ]]; then
            if ! command -v ydotool &>/dev/null; then
                log "KDE Plasma detected — installing ydotool (required for text injection)"
                sudo apt-get install -y ydotool
            else
                ok "ydotool (KDE Wayland)"
            fi
        else
            if ! command -v ydotool &>/dev/null; then
                warn "ydotool not found — text injection falls back to wtype; install ydotool for compositor-agnostic support"
            else
                ok "ydotool (optional, detected)"
            fi
        fi
        if command -v ydotool &>/dev/null; then
            log "Enabling ydotoold user service..."
            systemctl --user enable --now ydotoold || warn "Could not enable ydotoold — run: systemctl --user enable --now ydotoold"
        fi
```

**Step 3: Verify the script is syntactically valid**

Run: `bash -n setup.sh`
Expected: no output (clean parse).

**Step 4: Commit**

```bash
git add setup.sh
git commit -m "feat: install ydotool + enable ydotoold in setup.sh for KDE Wayland"
```

---

### Task 6: Update `README.md`

Three changes: features bullet, install instructions, troubleshooting section.

**Files:**
- Modify: `README.md:14` (features), `README.md:42-54` (install), `README.md:206` (diagram), `README.md:211-256` (troubleshooting)

**Step 1: Update the features bullet**

Change line 14:
```markdown
- ⌨️ **Automatic typing** into any focused text field (Wayland: `wtype`, X11: `xdotool`)
```
to:
```markdown
- ⌨️ **Automatic typing** into any focused text field — `ydotool` (all compositors), `wtype` (Wayland), `xdotool` (X11); auto-detected at startup
```

**Step 2: Add ydotool to the Arch install block**

After `sudo pacman -S wtype wl-clipboard` in the Arch block, add a new line:
```bash
# KDE Wayland — ydotool required; optional on other Wayland compositors:
sudo pacman -S ydotool
systemctl --user enable --now ydotoold
```

After `sudo apt install wtype wl-clipboard` in the Debian block, add:
```bash
# KDE Wayland — ydotool required; optional on other Wayland compositors:
sudo apt install ydotool
systemctl --user enable --now ydotoold
```

**Step 3: Update the architecture diagram**

Change the last line of the diagram (line 206):
```
wtype (Wayland) / xdotool (X11) → active window
```
to:
```
ydotool (all compositors) / wtype (Wayland) / xdotool (X11) → active window
```

**Step 4: Add two troubleshooting entries**

After the `**Text is not typed (Wayland)**` block (around line 232), insert:

```markdown
**Text not typed on KDE Plasma (Wayland)**
KDE does not implement the `zwlr_virtual_keyboard_v1` protocol used by `wtype`.
`whisper-type` auto-detects `ydotool` and uses it instead — install it and enable the daemon:
```bash
# Arch:
sudo pacman -S ydotool
# Debian/Ubuntu:
sudo apt install ydotool
# Enable the daemon (once per login session setup):
systemctl --user enable --now ydotoold
```
After installation, `whisper-type` picks it up automatically — no config change needed.

**`ydotool: Cannot connect to ydotoold`**
The daemon is not running. Enable it permanently:
```bash
systemctl --user enable --now ydotoold
```
```

**Step 5: Verify Markdown is well-formed**

Run: `grep -c '```' README.md`
Expected: an even number (all code fences are paired).

**Step 6: Commit**

```bash
git add README.md
git commit -m "docs: document ydotool backend in README (features, install, troubleshooting)"
```

---

### Task 7: Final verification

**Step 1: Full test suite**

Run: `cargo test`
Expected: 81 tests, 0 failures.

**Step 2: Clippy clean**

Run: `cargo clippy -- -D warnings`
Expected: no warnings.

**Step 3: Release build**

Run: `cargo build --release`
Expected: compiles successfully; no new crate dependencies.

**Step 4: Check test count matches plan**

Run: `cargo test 2>&1 | grep "^test result"`
Expected: lines summing to 81 passed tests.

---

## Summary of files changed

| File | Changes |
|------|---------|
| `src/typer.rs` | `Backend::Ydotool` variant; updated `probe_tool`, `Typer::new`, `type_text`; new `type_with_ydotool`, `type_with_clipboard_ydotool`; extended `set_clipboard`; 3 new unit tests |
| `setup.sh` | KDE-aware ydotool install + `ydotoold` service activation in Arch and Debian Wayland blocks |
| `README.md` | Features bullet, install instructions, architecture diagram, 2 troubleshooting entries |
