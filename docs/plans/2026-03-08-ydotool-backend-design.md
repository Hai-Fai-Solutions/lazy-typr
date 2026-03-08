# Design: ydotool backend for compositor-agnostic text injection

**Date:** 2026-03-08
**Branch:** `feature/enable-kde-support`
**Status:** Draft

---

## Problem

`wtype` requires the `zwlr_virtual_keyboard_v1` Wayland protocol. KDE Plasma on Wayland does not
implement this protocol, so text injection silently fails on KDE sessions and the typer falls
through to the slower clipboard paste fallback. There is no way to configure a different tool —
the backend was selected purely by `WAYLAND_DISPLAY` being set.

---

## Design

### 1. Backend selection (`src/typer.rs` — `Typer::new`)

A third `Backend::Ydotool` variant is added to the existing `Backend` enum.

Detection priority (first match wins):

| Priority | Condition | Backend | Tool |
|---|---|---|---|
| 1 | `ydotool --help` exits 0 | `Ydotool` | `ydotool type` |
| 2 | `WAYLAND_DISPLAY` set | `Wayland` | `wtype` |
| 3 | fallback | `X11` | `xdotool` |

`ydotool` is probed first because it works on all compositors and X11. Users without `ydotool`
installed are completely unaffected — the detection falls through to existing logic.

`probe_tool()` is already generic; a single new match arm is sufficient:

```rust
Backend::Ydotool => ("ydotool", "--help"),
```

### 2. Direct typing (`type_with_ydotool`)

```
ydotool type --key-delay=0 --key-hold=0 -- <text>
```

- `--key-delay=0` / `--key-hold=0`: zero inter-key delay, matching xdotool and wtype behaviour
- `--`: prevents text starting with `-` from being parsed as a flag
- Requires `ydotoold` daemon running; if it is not, the command exits non-zero and the
  clipboard fallback is invoked

### 3. Clipboard fallback (`type_with_clipboard_ydotool`)

Same three-step pattern used by the X11 and Wayland fallbacks:

1. Save current clipboard
2. Write text to clipboard via `wl-copy` (Wayland) / `xclip` / `arboard`
3. Send Ctrl+V: `ydotool key ctrl+v`
4. Restore saved clipboard

`ydotool key ctrl+v` uses the same tool already selected for the session, avoiding an additional
`wtype` or `xdotool` dependency in the fallback path.

`set_clipboard()` already tries `wl-copy` first for `Backend::Wayland`; this is extended to
`Backend::Ydotool` via:

```rust
if matches!(self.backend, Backend::Wayland | Backend::Ydotool) { ... }
```

### 4. Setup (`setup.sh`)

Inside the Wayland detection block, after `wtype` and `wl-clipboard`:

- If `XDG_CURRENT_DESKTOP` contains `kde`: install `ydotool` automatically (required)
- Otherwise: warn that `ydotool` is optional but recommended
- If `ydotool` is installed: enable `ydotoold` via `systemctl --user enable --now ydotoold`

`$XDG_CURRENT_DESKTOP` is set by the login manager and is the standard variable for compositor
identification. Case-insensitive match (`,,` parameter expansion) handles `KDE`, `kde`, `KDE Plasma`.

### 5. Documentation (`README.md`)

- Features bullet updated to list all three backends and their compositor scope
- Install instructions added for `ydotool` + `ydotoold` service for Arch and Debian
- Two troubleshooting entries:
  - *Text not typed (KDE Wayland)* — root cause, fix, auto-detection explanation
  - *`ydotool: Cannot connect to ydotoold`* — service enable command

---

## Error handling

| Situation | Behaviour |
|---|---|
| `ydotool` not on PATH | probe returns false → falls through to wtype/xdotool detection |
| `ydotoold` not running | `ydotool type` exits non-zero → clipboard fallback invoked; logged as info |
| `ydotool key ctrl+v` fails in fallback | error propagated to caller; logged as error in typer thread |
| `ydotool` on PATH but broken | same as "not running" — non-zero exit → clipboard fallback |

---

## Security considerations

### New attack surface

- **`ydotool` invocation**: `ydotool type -- <text>` passes Whisper output as an argument.
  The `--` separator prevents the text from being interpreted as flags. The text is not passed
  through a shell; it is the direct argument vector of `Command::new("ydotool")`, so shell
  injection is not possible. Risk: **none** (same as existing xdotool/wtype invocations).

- **`ydotool key ctrl+v`**: the key argument is a hard-coded string literal, not derived from
  user or Whisper output. Risk: **none**.

- **`/dev/uinput` access**: ydotool/ydotoold require write access to `/dev/uinput`. This is
  granted by the `input` group or a udev rule — the same requirement as PTT (`evdev`). No new
  permissions are needed by `whisper-type` itself; the daemon handles the privilege. Risk: **low**
  (ydotoold is a system-level daemon managed by the user, not whisper-type).

### No change to existing surfaces

- Whisper output sanitisation path (`transcriber.rs` → `typer.rs`) is unchanged
- All three backends use identical text-forwarding logic — no new parsing or interpretation
- Clipboard save/restore path is unchanged

---

## Testing

### Unit tests (no hardware required — all pass in CI)

All 78 existing unit tests must pass unchanged:

- `test_dry_run_*` — ydotool never called in dry-run mode
- `test_tool_available_false_when_binary_missing` — PATH is cleared; ydotool probe fails;
  falls through to Wayland/X11 detection; `tool_available` is correctly `false`
- `test_type_text_skips_direct_typing_when_unavailable` — constructs `Typer` with
  `Backend::Wayland` directly; unaffected by new variant

No new unit tests are strictly required — the ydotool path is structurally identical to the
existing wtype and xdotool paths, which are already tested.

### Integration / CI

- `cargo clippy -- -D warnings` must pass (all new match arms are exhaustive)
- `cargo build --release` must succeed (no new dependencies)
- `cargo test` — all 66 tests must pass

### Manual / hardware tests (out of scope for CI)

- KDE Wayland: install `ydotool`, enable `ydotoold`, run `whisper-type --dry-run`;
  log must show `ydotool detected — using ydotool (compositor-agnostic)`
- Without `ydotool`: log must show `Wayland display detected — using wtype` (unchanged)
- X11: log must show `xdotool` (unchanged)
- `ydotoold` stopped: `ydotool type` fails; fallback log line appears; clipboard paste works

---

## Files changed

| File | Change |
|---|---|
| `src/typer.rs` | Add `Backend::Ydotool`; probe; detection order; `type_with_ydotool()`; `type_with_clipboard_ydotool()`; extend `set_clipboard()` wl-copy path |
| `setup.sh` | Auto-install `ydotool` on KDE Wayland; enable `ydotoold` user service (Arch + Debian) |
| `README.md` | Features bullet; install instructions; two troubleshooting entries |