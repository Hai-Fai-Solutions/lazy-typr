# Design: Fix wtype Availability Check and Key-Combo Syntax

**Date:** 2026-03-06
**Branch:** `bugfix/fix-wtype`
**Status:** Approved

## Problem

Two bugs in `src/typer.rs`:

1. **No availability check** — `Typer` attempts to call `wtype` (or `xdotool`) on every transcription without first verifying the binary is present on `PATH`. When the binary is missing the call fails silently, logs `"wtype failed, falling back to clipboard paste"`, and the clipboard fallback itself then also fails.

2. **Wrong key-combo syntax in the clipboard fallback** — `type_with_clipboard_wayland` invokes `wtype -k ctrl+v`, which is not valid wtype syntax. wtype requires modifier keys to be pressed and released explicitly: `wtype -M ctrl -k v -m ctrl`.

## Design

### 1. Probe tool availability at startup

Add a `tool_available: bool` field to `Typer`. In `Typer::new()`, after determining the backend, probe the tool with a version query:

- Wayland: `wtype --version`
- X11: `xdotool version`

If the command cannot be spawned or exits non-zero, set `tool_available = false` and emit a single `warn!` log:

```
warn!("wtype not found on PATH; direct typing disabled, clipboard paste will be used")
```

In `type_text`, when `!self.tool_available`, skip the direct-typing attempt and go straight to the clipboard path.

### 2. Fix wtype key-combo syntax

Change the `wtype` invocation in `type_with_clipboard_wayland` from:

```rust
.args(["-k", "ctrl+v"])
```

to:

```rust
.args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
```

### 3. Error handling

| Situation | Behaviour |
|---|---|
| Tool not on PATH at startup | `warn!` once; clipboard path used for all subsequent calls |
| Tool present but exits non-zero | existing `info!` fallback log retained |
| Clipboard paste also fails | existing `anyhow::bail!` retained |

### 4. Testing

- Existing dry-run unit tests are unaffected.
- New unit test: construct `Typer::new(false)` with `PATH` set to an empty directory; assert `typer.tool_available == false`.
- New unit test: verify the wtype key-combo args are `["-M", "ctrl", "-k", "v", "-m", "ctrl"]` (test the args array directly or via a helper).

## Files changed

- `src/typer.rs` — all changes confined to this file
