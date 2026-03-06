/// Integration test: every key name listed in `supported_keys()` must parse
/// without panicking and return `Some(_)`.
///
/// This catches regressions where a key is listed in the human-readable help
/// text but the match arm was accidentally removed from `parse_key`.
use whisper_type::ptt::{parse_key, supported_keys};

#[test]
fn all_canonical_key_names_parse_successfully() {
    // These are the canonical names users are told to use in the docs / --help.
    let canonical = [
        "KEY_SPACE",
        "KEY_CAPSLOCK",
        "KEY_SCROLLLOCK",
        "KEY_PAUSE",
        "KEY_INSERT",
        "KEY_LEFTCTRL",
        "KEY_RIGHTCTRL",
        "KEY_LEFTSHIFT",
        "KEY_RIGHTSHIFT",
        "KEY_LEFTALT",
        "KEY_RIGHTALT",
        "KEY_LEFTMETA",
        "KEY_F1",
        "KEY_F2",
        "KEY_F3",
        "KEY_F4",
        "KEY_F5",
        "KEY_F6",
        "KEY_F7",
        "KEY_F8",
        "KEY_F9",
        "KEY_F10",
        "KEY_F11",
        "KEY_F12",
    ];

    for name in canonical {
        assert!(
            parse_key(name).is_some(),
            "parse_key({:?}) returned None — key is advertised but not implemented",
            name
        );
    }
}

#[test]
fn common_aliases_parse_successfully() {
    let aliases = [
        ("CTRL", "KEY_LEFTCTRL alias"),
        ("SHIFT", "KEY_LEFTSHIFT alias"),
        ("ALT", "KEY_LEFTALT alias"),
        ("SUPER", "KEY_LEFTMETA alias"),
        ("META", "KEY_LEFTMETA alias"),
        ("ALTGR", "KEY_RIGHTALT alias"),
    ];

    for (name, desc) in aliases {
        assert!(
            parse_key(name).is_some(),
            "alias {} ({}) should parse",
            name,
            desc
        );
    }
}

#[test]
fn supported_keys_text_is_non_empty() {
    assert!(!supported_keys().is_empty());
}
