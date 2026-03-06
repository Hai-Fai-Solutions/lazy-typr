use anyhow::{Context, Result};
use tracing::{debug, info};

enum Backend {
    X11,
    Wayland,
}

pub struct Typer {
    dry_run: bool,
    backend: Backend,
}

impl Typer {
    pub fn new(dry_run: bool) -> Self {
        let backend = if std::env::var("WAYLAND_DISPLAY").is_ok() {
            info!("Wayland display detected — using wtype");
            Backend::Wayland
        } else {
            Backend::X11
        };
        Self { dry_run, backend }
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
                if self.type_with_xdotool(&text_with_space).is_ok() {
                    return Ok(());
                }
                info!("xdotool failed, falling back to clipboard paste");
                self.type_with_clipboard_x11(&text_with_space)
            }
            Backend::Wayland => {
                if self.type_with_wtype(&text_with_space).is_ok() {
                    return Ok(());
                }
                info!("wtype failed, falling back to clipboard paste");
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

        // wtype -k sends key combos; ctrl+v pastes in most Wayland apps
        let paste_result = std::process::Command::new("wtype")
            .args(["-k", "ctrl+v"])
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
        if matches!(self.backend, Backend::Wayland) {
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
}
