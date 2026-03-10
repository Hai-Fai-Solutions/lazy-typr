use whisper_type::config::{Config, Task};

#[test]
fn no_cli_flags_keeps_config_values() {
    let mut cfg = Config {
        language: "fr".to_string(),
        silence_threshold_ms: 1200,
        ..Config::default()
    };
    cfg.apply_language_override(None);

    assert_eq!(cfg.language, "fr");
    assert_eq!(cfg.silence_threshold_ms, 1200);
}

#[test]
fn cli_language_overrides_config() {
    let mut cfg = Config {
        language: "fr".to_string(),
        ..Config::default()
    };
    cfg.apply_language_override(Some("en".to_string()));

    assert_eq!(cfg.language, "en");
}

#[test]
fn cli_without_language_does_not_mutate_language() {
    let mut cfg = Config {
        language: "es".to_string(),
        ..Config::default()
    };
    cfg.apply_language_override(None);

    assert_eq!(cfg.language, "es");
}

#[test]
fn cli_without_silence_ms_does_not_mutate_silence_threshold() {
    let mut cfg = Config {
        silence_threshold_ms: 1500,
        ..Config::default()
    };
    cfg.apply_silence_override(None);
    assert_eq!(cfg.silence_threshold_ms, 1500);
}

#[test]
fn cli_silence_ms_overrides_config() {
    let mut cfg = Config {
        silence_threshold_ms: 1500,
        ..Config::default()
    };
    cfg.apply_silence_override(Some(400));
    assert_eq!(cfg.silence_threshold_ms, 400);
}

#[test]
fn no_cli_task_keeps_config_task() {
    let mut cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    cfg.apply_whisper_task_override(None);
    assert_eq!(cfg.whisper_task, Task::Translate);
}

#[test]
fn cli_task_overrides_config_task() {
    let mut cfg = Config {
        whisper_task: Task::Translate,
        ..Config::default()
    };
    cfg.apply_whisper_task_override(Some(Task::Transcribe));
    assert_eq!(cfg.whisper_task, Task::Transcribe);
}
