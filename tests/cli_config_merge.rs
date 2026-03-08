use whisper_type::cli_overrides::{apply_cli_overrides, CliOverrides};
use whisper_type::config::Config;

#[test]
fn no_cli_flags_keeps_config_values() {
    let mut cfg = Config {
        language: "fr".to_string(),
        silence_threshold_ms: 1200,
        ..Config::default()
    };
    let overrides = CliOverrides { language: None };

    apply_cli_overrides(&mut cfg, &overrides);

    assert_eq!(cfg.language, "fr");
    assert_eq!(cfg.silence_threshold_ms, 1200);
}

#[test]
fn cli_language_overrides_config() {
    let mut cfg = Config {
        language: "fr".to_string(),
        ..Config::default()
    };
    let overrides = CliOverrides {
        language: Some("en".to_string()),
    };

    apply_cli_overrides(&mut cfg, &overrides);

    assert_eq!(cfg.language, "en");
}

#[test]
fn cli_without_language_does_not_mutate_language() {
    let mut cfg = Config {
        language: "es".to_string(),
        ..Config::default()
    };
    let overrides = CliOverrides { language: None };

    apply_cli_overrides(&mut cfg, &overrides);

    assert_eq!(cfg.language, "es");
}
