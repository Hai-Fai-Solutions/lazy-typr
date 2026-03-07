# Plan: ydotool backend for compositor-agnostic text injection

## Context

`wtype` (the current Wayland typing backend) requires the `zwlr_virtual_keyboard_v1` Wayland
protocol. This protocol is implemented by wlroots-based compositors (Sway, Hyprland, Wayfire)
but **not** by KDE Plasma on Wayland, which uses its own input protocols. As a result, text
injection silently fails on KDE Wayland and falls through to the clipboard paste fallback.

`ydotool` solves this by writing directly to `/dev/uinput` via the `ydotoold` user-space daemon.
It is compositor-agnostic — it works on KDE Wayland, GNOME Wayland, wlroots compositors, and X11.

---

## Branch

```bash
git checkout feature/gpu-acceleration
git checkout -b feature/ydotool-backend
```

---

## Detection order

```
1. ydotool on PATH  →  Backend::Ydotool   (compositor-agnostic, highest priority)
2. WAYLAND_DISPLAY  →  Backend::Wayland   (wtype, wlroots compositors)
3. fallback         →  Backend::X11       (xdotool)
```

`ydotool` is probed first. If it is not installed, the existing wtype/xdotool detection is
completely unchanged — no regressions for current users.

---

## Changes

### 1. [src/typer.rs](../../src/typer.rs)

#### 1a. `Backend` enum — add `Ydotool` variant

```rust
enum Backend {
    X11,
    Wayland,
    /// ydotool writes to /dev/uinput via the ydotoold daemon.
    /// Works on any compositor (including KDE Wayland) and on X11.
    Ydotool,
}
```

#### 1b. `probe_tool()` — add Ydotool arm

```rust
Backend::Ydotool => ("ydotool", "--help"),
```

`ydotool --help` exits 0 regardless of whether `ydotoold` is running, making it a reliable
binary-presence probe. Actual typing failure (no daemon) is handled at call time and falls
through to the clipboard fallback.

#### 1c. `Typer::new()` — prepend ydotool probe

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

#### 1d. `type_text()` — add `Backend::Ydotool` arm

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

#### 1e. Add `type_with_ydotool()`

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

`--key-delay=0` and `--key-hold=0` match the zero-delay behaviour of xdotool and wtype.

#### 1f. Add `type_with_clipboard_ydotool()`

```rust
fn type_with_clipboard_ydotool(&self, text: &str) -> Result<()> {
    let saved = self.get_clipboard();
    self.set_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(50));
    let paste_result = std::process::Command::new("ydotool")
        .args(["key", "ctrl+v"])
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

#### 1g. `set_clipboard()` — include Ydotool in wl-copy path

```rust
if matches!(self.backend, Backend::Wayland | Backend::Ydotool) {
    // try wl-copy ...
}
```

---

### 2. [setup.sh](../../setup.sh)

Added inside the Wayland detection block (`if [ "$XDG_SESSION_TYPE" = "wayland" ] || ...`),
after `wtype` and `wl-clipboard`, for both Arch and Debian:

```bash
# ydotool: compositor-agnostic input injection (required for KDE Wayland)
if ! command -v ydotool &>/dev/null; then
    if [[ "${XDG_CURRENT_DESKTOP,,}" == *"kde"* ]]; then
        log "KDE Wayland detected — installing ydotool (wtype is not supported on KDE)"
        sudo pacman -S --needed --noconfirm ydotool   # or apt-get install -y ydotool
    else
        warn "ydotool not found — optional but recommended for KDE Wayland users"
    fi
else
    ok "ydotool"
fi
# Enable ydotoold user service if ydotool is now available
if command -v ydotool &>/dev/null; then
    if ! systemctl --user is-active --quiet ydotoold 2>/dev/null; then
        log "Enabling ydotoold user service..."
        systemctl --user enable --now ydotoold
    else
        ok "ydotoold service active"
    fi
fi
```

---

### 3. [README.md](../../README.md)

- **Features list** — update typing bullet to list all three backends
- **System dependencies** — add `ydotool` + `systemctl --user enable --now ydotoold` for Arch
  and Debian under the Wayland section
- **Troubleshooting** — two new entries:
  - *Text not typed (KDE Wayland)* — explains wtype limitation; directs user to install ydotool
  - *`ydotool: Cannot connect to ydotoold`* — directs user to enable the user service

---

## What does NOT change

- `src/config.rs` — no new fields; ydotool is auto-detected, no user configuration required
- `src/main.rs` — no CLI changes
- All existing tests — ydotool probe fails in CI (not on PATH), falling through to existing
  wtype/xdotool paths; all 66 tests pass unchanged

---

## Execution order

1. Create branch `feature/ydotool-backend` off `feature/gpu-acceleration`
2. Add `Backend::Ydotool` + probe + detection + methods to `src/typer.rs`
3. Update `setup.sh` (both Arch and Debian blocks)
4. Update `README.md`
5. `cargo fmt && cargo clippy -- -D warnings`
6. `cargo test` — all tests must pass
7. Open PR → `main`
