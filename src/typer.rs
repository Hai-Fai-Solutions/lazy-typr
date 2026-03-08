use anyhow::{Context, Result};
use tracing::{debug, info, warn};

/// Arguments for wtype to send Ctrl+V.
/// -M presses the modifier, -k sends the key, -m releases the modifier.
const WTYPE_PASTE_ARGS: &[&str] = &["-M", "ctrl", "-k", "v", "-m", "ctrl"];

enum Backend {
    X11,
    Wayland,
    /// ydotool writes to /dev/uinput via the ydotoold daemon.
    /// Works on any compositor (including KDE Wayland) and on X11.
    Ydotool,
}

pub struct Typer {
    dry_run: bool,
    backend: Backend,
    tool_available: bool,
}

impl Typer {
    /// Returns `true` if the backend binary is on PATH and exits successfully.
    /// Stdout and stderr are suppressed; the probe exits quickly via `--version`.
    fn probe_tool(backend: &Backend) -> bool {
        let (cmd, arg) = match backend {
            Backend::Ydotool => ("ydotool", "--version"),
            Backend::Wayland => ("wtype", "--version"),
            Backend::X11 => ("xdotool", "version"),
        };
        std::process::Command::new(cmd)
            .arg(arg)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Returns `true` if the ydotoold daemon socket is reachable.
    /// ydotool is useless without the daemon even if the binary is installed.
    fn probe_ydotoold_daemon() -> bool {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let base = std::path::Path::new(&runtime_dir);
            // Default socket path used by ydotoold out of the box
            if base.join(".ydotool_socket").exists() {
                return true;
            }
            // Alternative path used by some distro service units
            if base.join("ydotoold/ydotoold.sock").exists() {
                return true;
            }
        }
        // System-wide service socket
        std::path::Path::new("/run/ydotoold/ydotoold.sock").exists()
    }

    /// Returns `true` when running under KDE Plasma.
    /// wtype does not work on KDE Plasma Wayland (no wlr-virtual-keyboard support),
    /// so we need ydotool there instead.
    fn is_kde_plasma() -> bool {
        std::env::var("XDG_CURRENT_DESKTOP")
            .map(|v| v.to_ascii_uppercase().contains("KDE"))
            .unwrap_or(false)
            || std::env::var("KDE_FULL_SESSION").is_ok()
    }

    pub fn new(dry_run: bool) -> Self {
        let on_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

        let backend = if on_wayland {
            let kde = Self::is_kde_plasma();
            let ydotool_ready = Self::probe_ydotoold_daemon();

            if kde {
                // KDE Plasma Wayland: wtype has no wlr-virtual-keyboard support.
                // Require the ydotoold daemon to be running; warn loudly if it isn't.
                if ydotool_ready {
                    info!("KDE Plasma Wayland detected — using ydotool (ydotoold daemon found)");
                    Backend::Ydotool
                } else {
                    warn!(
                        "KDE Plasma Wayland detected but ydotoold daemon not found. \
                         wtype will not work on KDE. Start ydotoold or run: \
                         systemctl --user enable --now ydotoold"
                    );
                    Backend::Wayland
                }
            } else if Self::probe_tool(&Backend::Wayland) {
                // Non-KDE Wayland (GNOME, Sway, etc.): wtype works fine.
                info!("Wayland display detected — using wtype");
                Backend::Wayland
            } else if ydotool_ready {
                // wtype not installed but ydotoold is running — fall back to ydotool.
                info!("Wayland display detected, wtype unavailable — using ydotool");
                Backend::Ydotool
            } else {
                info!("Wayland display detected — using wtype");
                Backend::Wayland
            }
        } else {
            Backend::X11
        };

        let tool_available = if dry_run {
            true // no external tools needed in dry-run
        } else {
            let available = Self::probe_tool(&backend);
            if !available {
                let tool = match &backend {
                    Backend::Ydotool => "ydotool",
                    Backend::Wayland => "wtype",
                    Backend::X11 => "xdotool",
                };
                warn!("{} not found on PATH; clipboard paste will not work", tool);
            }
            available
        };

        Self {
            dry_run,
            backend,
            tool_available,
        }
    }

    /// Type the given text into the currently focused window.
    ///
    /// On ydotool: sets clipboard via wl-copy/arboard, then sends ctrl+v via ydotool key.
    ///             ydotool type is skipped entirely to avoid encoding issues with special chars.
    /// On Wayland: uses wtype, falls back to wl-clipboard + wtype key paste.
    /// On X11:     uses xdotool, falls back to xclip/xsel + ctrl+v paste.
    pub fn type_text(&self, text: &str) -> Result<()> {
        if self.dry_run {
            println!("{}", text);
            return Ok(());
        }

        // Add a trailing space so the next word doesn't merge
        let text_with_space = format!("{} ", text);

        debug!("Typing: {:?}", text_with_space);

        match self.backend {
            Backend::Ydotool => {
                // Always use clipboard + ctrl+v; ydotool type has encoding issues with
                // special/non-ASCII characters so we skip it entirely.
                self.type_with_clipboard_ydotool(&text_with_space)
            }
            Backend::X11 => {
                if self.tool_available && self.type_with_xdotool(&text_with_space).is_ok() {
                    return Ok(());
                }
                if self.tool_available {
                    info!("xdotool failed, falling back to clipboard paste");
                }
                self.type_with_clipboard_x11(&text_with_space)
            }
            Backend::Wayland => {
                if self.tool_available && self.type_with_wtype(&text_with_space).is_ok() {
                    return Ok(());
                }
                if self.tool_available {
                    info!("wtype failed, falling back to clipboard paste");
                }
                self.type_with_clipboard_wayland(&text_with_space)
            }
        }
    }

    fn type_with_xdotool(&self, text: &str) -> Result<()> {
        // --clearmodifiers: release Shift/Ctrl/etc. before typing
        // --delay 0: no inter-key delay (faster)
        let output = std::process::Command::new("xdotool")
            .args(["type", "--clearmodifiers", "--delay", "0", "--", text])
            .output()
            .context("xdotool not found. Install with: sudo pacman -S xdotool")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("xdotool failed: {}", stderr)
        }
    }

    fn type_with_wtype(&self, text: &str) -> Result<()> {
        let output = std::process::Command::new("wtype")
            .args(["--", text])
            .output()
            .context("wtype not found. Install with: sudo pacman -S wtype")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("wtype failed: {}", stderr)
        }
    }

    fn type_with_clipboard_ydotool(&self, text: &str) -> Result<()> {
        let saved = self.get_clipboard();

        self.set_clipboard(text)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // ydotool key uses Linux input keycode notation: "KEYCODE:VALUE"
        // 29 = KEY_LEFTCTRL, 47 = KEY_V; value 1 = press, 0 = release
        let paste_result = std::process::Command::new("ydotool")
            .args(["key", "29:1", "47:1", "47:0", "29:0"])
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

    fn type_with_clipboard_x11(&self, text: &str) -> Result<()> {
        let saved = self.get_clipboard();

        self.set_clipboard(text)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        let paste_result = std::process::Command::new("xdotool")
            .args(["key", "--clearmodifiers", "ctrl+v"])
            .status();

        if let Ok(saved_text) = saved {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = self.set_clipboard(&saved_text);
        }

        paste_result
            .context("xdotool key failed")?
            .success()
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("xdotool key ctrl+v failed"))
    }

    fn type_with_clipboard_wayland(&self, text: &str) -> Result<()> {
        let saved = self.get_clipboard();

        self.set_clipboard(text)?;
        std::thread::sleep(std::time::Duration::from_millis(50));

        // wtype modifier syntax: -M presses a modifier, -k sends the key, -m releases the modifier
        let paste_result = std::process::Command::new("wtype")
            .args(WTYPE_PASTE_ARGS)
            .status();

        if let Ok(saved_text) = saved {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = self.set_clipboard(&saved_text);
        }

        paste_result
            .context("wtype key failed")?
            .success()
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("wtype ctrl+v failed"))
    }

    fn set_clipboard(&self, text: &str) -> Result<()> {
        // Try wl-copy first on Wayland / ydotool (ydotool is typically used on Wayland)
        if matches!(self.backend, Backend::Wayland | Backend::Ydotool) {
            let r = std::process::Command::new("wl-copy")
                .stdin(std::process::Stdio::piped())
                .spawn();
            if let Ok(mut child) = r {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()?;
                return Ok(());
            }
        }

        // Try xclip (X11)
        let r = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn();

        if let Ok(mut child) = r {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            return Ok(());
        }

        // Try xsel (X11)
        let r = std::process::Command::new("xsel")
            .args(["--clipboard", "--input"])
            .stdin(std::process::Stdio::piped())
            .spawn();

        if let Ok(mut child) = r {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            return Ok(());
        }

        // Fallback: arboard (supports both X11 and Wayland)
        let mut clipboard = arboard::Clipboard::new().context("Failed to open clipboard")?;
        clipboard
            .set_text(text)
            .context("Failed to set clipboard text")?;
        Ok(())
    }

    fn get_clipboard(&self) -> Result<String> {
        let mut clipboard = arboard::Clipboard::new()?;
        Ok(clipboard.get_text().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In dry-run mode, `type_text` must succeed without calling any external tool.
    #[test]
    fn test_dry_run_type_text_succeeds() {
        let typer = Typer::new(true);
        assert!(typer.type_text("Hello World").is_ok());
    }

    /// The trailing space must be added even in dry-run (observable via stdout in integration,
    /// but here we just confirm the call doesn't error on edge cases).
    #[test]
    fn test_dry_run_empty_string() {
        let typer = Typer::new(true);
        assert!(typer.type_text("").is_ok());
    }

    #[test]
    fn test_dry_run_unicode_text() {
        let typer = Typer::new(true);
        assert!(typer.type_text("Größe 42 — naïve café").is_ok());
    }

    #[test]
    fn test_dry_run_multiline_text() {
        let typer = Typer::new(true);
        assert!(typer.type_text("line one\nline two").is_ok());
    }

    /// Ensure the clipboard-wayland fallback uses correct wtype key syntax.
    #[test]
    fn test_wtype_key_combo_args_are_correct() {
        // The correct wtype invocation for Ctrl+V is:
        //   wtype -M ctrl -k v -m ctrl
        // NOT:
        //   wtype -k ctrl+v
        assert_eq!(
            WTYPE_PASTE_ARGS,
            &["-M", "ctrl", "-k", "v", "-m", "ctrl"],
            "wtype paste args must use modifier syntax, not compound key strings"
        );
    }

    /// When tool is unavailable, type_text must skip direct typing and go straight
    /// to clipboard paste (which may fail in a test environment — that's acceptable).
    #[test]
    fn test_type_text_skips_direct_typing_when_unavailable() {
        // Construct Typer with tool_available=false directly to test this code path.
        let typer = Typer {
            dry_run: false,
            backend: Backend::Wayland,
            tool_available: false,
        };
        // We can't easily intercept subprocess calls, so this is a smoke test:
        // it must not panic, and must not attempt to call wtype for direct typing.
        let _ = typer.type_text("hello");
    }

    /// When wtype/xdotool is not on PATH, tool_available must be false.
    #[test]
    fn test_tool_available_false_when_binary_missing() {
        let tmp = std::env::temp_dir().join("empty_path_for_test");
        std::fs::create_dir_all(&tmp).unwrap();
        let original = std::env::var("PATH").unwrap_or_default();

        // Drop guard restores PATH even if the test panics
        struct PathGuard(String);
        impl Drop for PathGuard {
            fn drop(&mut self) {
                unsafe { std::env::set_var("PATH", &self.0) };
            }
        }
        let _guard = PathGuard(original);

        // SAFETY: intentional env mutation scoped to this test; restored by _guard
        unsafe { std::env::set_var("PATH", &tmp) };
        let typer = Typer::new(false);
        assert!(
            !typer.tool_available,
            "tool_available should be false when binary is missing"
        );
    }
}
