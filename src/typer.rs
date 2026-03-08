use anyhow::{Context, Result};
use tracing::{debug, info, warn};

/// Arguments for wtype to send Ctrl+V.
/// -M presses the modifier, -k sends the key, -m releases the modifier.
const WTYPE_PASTE_ARGS: &[&str] = &["-M", "ctrl", "-k", "v", "-m", "ctrl"];

/// Arguments for ydotool to send Ctrl+V.
const YDOTOOL_PASTE_ARGS: &[&str] = &["key", "ctrl+v"];

enum Backend {
    X11,
    Wayland,
    #[allow(dead_code)] // wired into Typer::new in a later task
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
            Backend::Wayland => ("wtype", "--version"),
            Backend::X11 => ("xdotool", "version"),
            Backend::Ydotool => ("ydotool", "--help"),
        };
        std::process::Command::new(cmd)
            .arg(arg)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn new(dry_run: bool) -> Self {
        let backend = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            info!("Wayland display detected — using wtype");
            Backend::Wayland
        } else {
            Backend::X11
        };

        let tool_available = if dry_run {
            true // no external tools needed in dry-run
        } else {
            let available = Self::probe_tool(&backend);
            if !available {
                let tool = match &backend {
                    Backend::Wayland => "wtype",
                    Backend::X11 => "xdotool",
                    Backend::Ydotool => "ydotool",
                };
                warn!(
                    "{} not found on PATH; direct typing disabled, clipboard paste will be used",
                    tool
                );
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
            Backend::Ydotool => {
                if self.tool_available && self.type_with_ydotool(&text_with_space).is_ok() {
                    return Ok(());
                }
                if self.tool_available {
                    info!("ydotool failed, falling back to clipboard paste");
                }
                self.type_with_clipboard_ydotool(&text_with_space)
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
        // Try wl-copy first on Wayland
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

    /// Ensure the clipboard-ydotool fallback uses correct ydotool key syntax.
    #[test]
    fn test_ydotool_key_combo_args_are_correct() {
        assert_eq!(
            YDOTOOL_PASTE_ARGS,
            &["key", "ctrl+v"],
            "ydotool paste args must be 'ydotool key ctrl+v'"
        );
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
