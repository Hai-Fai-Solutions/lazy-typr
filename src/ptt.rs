use anyhow::{Context, Result};
use evdev::{EventType, Key};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::{debug, error, info, warn};

/// Parse a PTT key name to an evdev Key.
///
/// Accepts names with or without the `KEY_` prefix, case-insensitive.
/// Examples: "KEY_SPACE", "SPACE", "key_capslock", "F1", "LEFTCTRL"
pub fn parse_key(name: &str) -> Option<Key> {
    let upper = name.to_uppercase();
    let bare = upper.strip_prefix("KEY_").unwrap_or(&upper);

    match bare {
        // Common modifiers
        "LEFTCTRL" | "CTRL" => Some(Key::KEY_LEFTCTRL),
        "RIGHTCTRL" => Some(Key::KEY_RIGHTCTRL),
        "LEFTSHIFT" | "SHIFT" => Some(Key::KEY_LEFTSHIFT),
        "RIGHTSHIFT" => Some(Key::KEY_RIGHTSHIFT),
        "LEFTALT" | "ALT" => Some(Key::KEY_LEFTALT),
        "RIGHTALT" | "ALTGR" => Some(Key::KEY_RIGHTALT),
        "LEFTMETA" | "SUPER" | "META" => Some(Key::KEY_LEFTMETA),
        "RIGHTMETA" => Some(Key::KEY_RIGHTMETA),

        // Toggle keys
        "CAPSLOCK" => Some(Key::KEY_CAPSLOCK),
        "SCROLLLOCK" => Some(Key::KEY_SCROLLLOCK),
        "NUMLOCK" => Some(Key::KEY_NUMLOCK),

        // Special
        "SPACE" => Some(Key::KEY_SPACE),
        "PAUSE" => Some(Key::KEY_PAUSE),
        "INSERT" => Some(Key::KEY_INSERT),

        // Function keys
        "F1" => Some(Key::KEY_F1),
        "F2" => Some(Key::KEY_F2),
        "F3" => Some(Key::KEY_F3),
        "F4" => Some(Key::KEY_F4),
        "F5" => Some(Key::KEY_F5),
        "F6" => Some(Key::KEY_F6),
        "F7" => Some(Key::KEY_F7),
        "F8" => Some(Key::KEY_F8),
        "F9" => Some(Key::KEY_F9),
        "F10" => Some(Key::KEY_F10),
        "F11" => Some(Key::KEY_F11),
        "F12" => Some(Key::KEY_F12),

        _ => None,
    }
}

/// Return a human-readable list of supported PTT key names.
pub fn supported_keys() -> &'static str {
    "KEY_SPACE, KEY_CAPSLOCK, KEY_SCROLLLOCK, KEY_PAUSE, KEY_INSERT, \
     KEY_LEFTCTRL, KEY_RIGHTCTRL, KEY_LEFTSHIFT, KEY_RIGHTSHIFT, \
     KEY_LEFTALT, KEY_RIGHTALT, KEY_LEFTMETA, KEY_F1–KEY_F12"
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_key — prefix variants ──────────────────────────────────────────

    #[test]
    fn test_parse_key_with_prefix_uppercase() {
        assert_eq!(parse_key("KEY_SPACE"), Some(Key::KEY_SPACE));
        assert_eq!(parse_key("KEY_CAPSLOCK"), Some(Key::KEY_CAPSLOCK));
        assert_eq!(parse_key("KEY_PAUSE"), Some(Key::KEY_PAUSE));
        assert_eq!(parse_key("KEY_INSERT"), Some(Key::KEY_INSERT));
    }

    #[test]
    fn test_parse_key_without_prefix() {
        assert_eq!(parse_key("SPACE"), Some(Key::KEY_SPACE));
        assert_eq!(parse_key("CAPSLOCK"), Some(Key::KEY_CAPSLOCK));
        assert_eq!(parse_key("SCROLLLOCK"), Some(Key::KEY_SCROLLLOCK));
        assert_eq!(parse_key("NUMLOCK"), Some(Key::KEY_NUMLOCK));
    }

    #[test]
    fn test_parse_key_lowercase_accepted() {
        assert_eq!(parse_key("key_space"), Some(Key::KEY_SPACE));
        assert_eq!(parse_key("space"), Some(Key::KEY_SPACE));
        assert_eq!(parse_key("key_f1"), Some(Key::KEY_F1));
    }

    // ── parse_key — modifier aliases ──────────────────────────────────────────

    #[test]
    fn test_parse_key_ctrl_aliases() {
        assert_eq!(parse_key("CTRL"), Some(Key::KEY_LEFTCTRL));
        assert_eq!(parse_key("LEFTCTRL"), Some(Key::KEY_LEFTCTRL));
        assert_eq!(parse_key("RIGHTCTRL"), Some(Key::KEY_RIGHTCTRL));
    }

    #[test]
    fn test_parse_key_shift_aliases() {
        assert_eq!(parse_key("SHIFT"), Some(Key::KEY_LEFTSHIFT));
        assert_eq!(parse_key("LEFTSHIFT"), Some(Key::KEY_LEFTSHIFT));
        assert_eq!(parse_key("RIGHTSHIFT"), Some(Key::KEY_RIGHTSHIFT));
    }

    #[test]
    fn test_parse_key_alt_aliases() {
        assert_eq!(parse_key("ALT"), Some(Key::KEY_LEFTALT));
        assert_eq!(parse_key("LEFTALT"), Some(Key::KEY_LEFTALT));
        assert_eq!(parse_key("RIGHTALT"), Some(Key::KEY_RIGHTALT));
        assert_eq!(parse_key("ALTGR"), Some(Key::KEY_RIGHTALT));
    }

    #[test]
    fn test_parse_key_meta_aliases() {
        assert_eq!(parse_key("SUPER"), Some(Key::KEY_LEFTMETA));
        assert_eq!(parse_key("META"), Some(Key::KEY_LEFTMETA));
        assert_eq!(parse_key("LEFTMETA"), Some(Key::KEY_LEFTMETA));
        assert_eq!(parse_key("RIGHTMETA"), Some(Key::KEY_RIGHTMETA));
    }

    // ── parse_key — function keys ─────────────────────────────────────────────

    #[test]
    fn test_parse_key_function_keys() {
        let expected = [
            ("F1", Key::KEY_F1),
            ("F2", Key::KEY_F2),
            ("F3", Key::KEY_F3),
            ("F4", Key::KEY_F4),
            ("F5", Key::KEY_F5),
            ("F6", Key::KEY_F6),
            ("F7", Key::KEY_F7),
            ("F8", Key::KEY_F8),
            ("F9", Key::KEY_F9),
            ("F10", Key::KEY_F10),
            ("F11", Key::KEY_F11),
            ("F12", Key::KEY_F12),
        ];
        for (name, key) in expected {
            assert_eq!(parse_key(name), Some(key), "failed for {}", name);
            // Also with KEY_ prefix
            let prefixed = format!("KEY_{}", name);
            assert_eq!(parse_key(&prefixed), Some(key), "failed for {}", prefixed);
        }
    }

    // ── parse_key — unknown ───────────────────────────────────────────────────

    #[test]
    fn test_parse_key_unknown_returns_none() {
        assert_eq!(parse_key("KEY_UNKNOWN"), None);
        assert_eq!(parse_key("BANANA"), None);
        assert_eq!(parse_key(""), None);
        assert_eq!(parse_key("KEY_"), None);
    }

    // ── supported_keys ────────────────────────────────────────────────────────

    #[test]
    fn test_supported_keys_is_non_empty() {
        assert!(!supported_keys().is_empty());
    }
}

/// Spawn background threads that monitor `/dev/input` devices for the PTT key.
///
/// Sets `ptt_active` to `true` on keydown and `false` on keyup.
/// One thread is spawned per input device that reports the key.
///
/// Requires the process owner to be in the `input` group:
///   `sudo usermod -aG input $USER`  (then re-login)
pub fn spawn_ptt_monitor(
    key: Key,
    ptt_active: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    // Find all input devices that support the requested key
    let matching: Vec<_> = evdev::enumerate()
        .filter(|(_, dev)| {
            dev.supported_keys()
                .map(|keys| keys.contains(key))
                .unwrap_or(false)
        })
        .collect();

    if matching.is_empty() {
        anyhow::bail!(
            "No input device found that reports {:?}. \
             Make sure you are in the 'input' group: sudo usermod -aG input $USER",
            key
        );
    }

    info!(
        "PTT: monitoring {} input device(s) for {:?}",
        matching.len(),
        key
    );

    for (path, mut device) in matching {
        let ptt_active = ptt_active.clone();
        let running = running.clone();

        std::thread::Builder::new()
            .name(format!("ptt-{}", path.display()))
            .spawn(move || {
                debug!("PTT monitor started for {}", path.display());
                loop {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }

                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                if event.event_type() != EventType::KEY {
                                    continue;
                                }
                                if Key::new(event.code()) != key {
                                    continue;
                                }
                                match event.value() {
                                    1 => {
                                        // keydown
                                        debug!("PTT key pressed");
                                        ptt_active.store(true, Ordering::SeqCst);
                                    }
                                    0 => {
                                        // keyup
                                        debug!("PTT key released");
                                        ptt_active.store(false, Ordering::SeqCst);
                                    }
                                    _ => {} // autorepeat (value=2), ignore
                                }
                            }
                        }
                        Err(e) => {
                            warn!("PTT device error on {}: {}", path.display(), e);
                            // On persistent error (device disconnected etc.), clear flag and exit
                            ptt_active.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }
                debug!("PTT monitor stopped for {}", path.display());
            })
            .context("Failed to spawn PTT monitor thread")?;
    }

    Ok(())
}
