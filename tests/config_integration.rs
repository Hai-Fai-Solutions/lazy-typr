use serde::{Deserialize, Serialize};
/// Integration tests for config file lifecycle.
///
/// These tests write real files to a tempdir — no mocking.
use std::path::PathBuf;

// Re-implement only what we need so the tests are self-contained without
// relying on private helpers. We call the public `serde_json` API directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ConfigSnapshot {
    language: String,
    silence_threshold_ms: u64,
    min_speech_ms: u64,
    max_buffer_secs: f32,
    vad_threshold: f32,
    #[serde(default = "default_log_level")]
    log_level: String,
    #[serde(default)]
    ptt_key: Option<String>,
    #[serde(default)]
    translate: bool,
    model_path: PathBuf,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn write_and_read_back(snapshot: &ConfigSnapshot) -> ConfigSnapshot {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let json = serde_json::to_string_pretty(snapshot).unwrap();
    std::fs::write(&path, json).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&content).unwrap()
}

#[test]
fn config_full_roundtrip() {
    let original = ConfigSnapshot {
        language: "en".to_string(),
        silence_threshold_ms: 1200,
        min_speech_ms: 250,
        max_buffer_secs: 45.0,
        vad_threshold: 0.03,
        log_level: "debug".to_string(),
        ptt_key: Some("KEY_F10".to_string()),
        translate: true,
        model_path: PathBuf::from("/tmp/ggml-base.bin"),
    };

    let restored = write_and_read_back(&original);
    assert_eq!(restored, original);
}

#[test]
fn config_missing_optional_fields_use_defaults() {
    let json = r#"{
        "model_path": "/tmp/model.bin",
        "language": "de",
        "silence_threshold_ms": 800,
        "min_speech_ms": 300,
        "max_buffer_secs": 30.0,
        "vad_threshold": 0.01
    }"#;

    let cfg: ConfigSnapshot = serde_json::from_str(json).unwrap();
    assert!(cfg.ptt_key.is_none());
    assert_eq!(cfg.log_level, "info");
    assert!(!cfg.translate);
}

#[test]
fn config_file_survives_multiple_write_read_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let mut cfg = ConfigSnapshot {
        language: "de".to_string(),
        silence_threshold_ms: 800,
        min_speech_ms: 300,
        max_buffer_secs: 30.0,
        vad_threshold: 0.01,
        log_level: "info".to_string(),
        ptt_key: None,
        translate: false,
        model_path: PathBuf::from("/tmp/model.bin"),
    };

    for i in 0..3 {
        cfg.silence_threshold_ms = 800 + i * 100;
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        std::fs::write(&path, &json).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: ConfigSnapshot = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.silence_threshold_ms, 800 + i * 100);
    }
}

#[test]
fn config_invalid_json_returns_error() {
    let result: Result<ConfigSnapshot, _> = serde_json::from_str("{ not valid json }");
    assert!(result.is_err());
}
