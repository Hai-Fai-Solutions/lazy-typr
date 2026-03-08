use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the Whisper GGML model
    pub model_path: PathBuf,

    /// Optional audio device name
    pub device_name: Option<String>,

    /// Whisper language code ("de", "en", etc.)
    pub language: String,

    /// Milliseconds of silence before a speech segment is finalized
    pub silence_threshold_ms: u64,

    /// Minimum speech duration in ms to trigger transcription
    pub min_speech_ms: u64,

    /// Maximum recording buffer in seconds
    pub max_buffer_secs: f32,

    /// Energy threshold for voice activity detection (0.0 - 1.0)
    pub vad_threshold: f32,

    /// Log level: "error", "warn", "info", "debug", "trace"
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// PTT key name (e.g. "KEY_SPACE", "KEY_CAPSLOCK"). None = VAD mode.
    #[serde(default)]
    pub ptt_key: Option<String>,

    /// Use GPU (Vulkan) for Whisper inference. Default: false (CPU).
    #[serde(default)]
    pub use_gpu: bool,

    /// Vulkan GPU device index. 0 = first device. Only used when use_gpu = true.
    #[serde(default)]
    pub gpu_device: u32,

    /// Print to stdout instead of typing
    #[serde(skip)]
    pub dry_run: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let model_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whisper-type")
            .join("ggml-base.bin");

        Self {
            model_path,
            device_name: None,
            language: "de".to_string(),
            silence_threshold_ms: 800,
            min_speech_ms: 300,
            max_buffer_secs: 30.0,
            vad_threshold: 0.01,
            log_level: default_log_level(),
            ptt_key: None,
            use_gpu: false,
            gpu_device: 0,
            dry_run: false,
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("whisper-type")
            .join("config.json")
    }

    pub fn load_or_default() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = serde_json::from_str(&content)?;
            tracing::info!("Loaded config from {}", path.display());
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Like `load_or_default` but without logging (used before tracing is initialized).
    pub fn load_or_default_quiet() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        tracing::info!("Saved config to {}", path.display());
        Ok(())
    }

    /// Apply CLI language override only when it is explicitly provided.
    pub fn apply_language_override(&mut self, language: Option<String>) {
        if let Some(language) = language {
            self.language = language;
        }
    }

    /// Save to an explicit path — used in tests to avoid touching the real config dir.
    #[cfg(test)]
    fn save_to(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load from an explicit path — used in tests.
    #[cfg(test)]
    fn load_from(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Default values ────────────────────────────────────────────────────────

    #[test]
    fn test_default_language_is_german() {
        assert_eq!(Config::default().language, "de");
    }

    #[test]
    fn test_apply_language_override_with_none_keeps_existing_value() {
        let mut cfg = Config {
            language: "fr".to_string(),
            ..Config::default()
        };

        cfg.apply_language_override(None);

        assert_eq!(cfg.language, "fr");
    }

    #[test]
    fn test_apply_language_override_with_some_replaces_value() {
        let mut cfg = Config {
            language: "fr".to_string(),
            ..Config::default()
        };

        cfg.apply_language_override(Some("en".to_string()));

        assert_eq!(cfg.language, "en");
    }

    #[test]
    fn test_default_log_level_is_info() {
        assert_eq!(Config::default().log_level, "info");
    }

    #[test]
    fn test_default_ptt_key_is_none() {
        assert!(Config::default().ptt_key.is_none());
    }

    #[test]
    fn test_default_vad_threshold_positive() {
        let cfg = Config::default();
        assert!(cfg.vad_threshold > 0.0, "vad_threshold should be positive");
        assert!(cfg.vad_threshold < 1.0, "vad_threshold should be < 1.0");
    }

    #[test]
    fn test_default_silence_threshold_ms_positive() {
        assert!(Config::default().silence_threshold_ms > 0);
    }

    #[test]
    fn test_default_dry_run_is_false() {
        assert!(!Config::default().dry_run);
    }

    #[test]
    fn test_default_use_gpu_is_false() {
        assert!(!Config::default().use_gpu);
    }

    #[test]
    fn test_default_gpu_device_is_zero() {
        assert_eq!(Config::default().gpu_device, 0);
    }

    #[test]
    fn test_gpu_device_round_trips_through_json() {
        let cfg = Config {
            gpu_device: 2,
            ..Config::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.gpu_device, 2);
    }

    #[test]
    fn test_gpu_device_absent_in_legacy_json_defaults_to_zero() {
        let json = r#"{
            "model_path": "/tmp/model.bin",
            "language": "de",
            "silence_threshold_ms": 800,
            "min_speech_ms": 300,
            "max_buffer_secs": 30.0,
            "vad_threshold": 0.01
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.gpu_device, 0);
    }

    #[test]
    fn test_use_gpu_round_trips_through_json() {
        let cfg = Config {
            use_gpu: true,
            ..Config::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert!(restored.use_gpu);
    }

    #[test]
    fn test_use_gpu_absent_in_legacy_json_defaults_to_false() {
        let json = r#"{
            "model_path": "/tmp/model.bin",
            "language": "de",
            "silence_threshold_ms": 800,
            "min_speech_ms": 300,
            "max_buffer_secs": 30.0,
            "vad_threshold": 0.01
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert!(!cfg.use_gpu);
    }

    // ── Serialization round-trip ──────────────────────────────────────────────

    #[test]
    fn test_serialization_roundtrip() {
        let original = Config {
            language: "en".to_string(),
            silence_threshold_ms: 1000,
            min_speech_ms: 400,
            max_buffer_secs: 20.0,
            vad_threshold: 0.05,
            log_level: "debug".to_string(),
            ptt_key: Some("KEY_F9".to_string()),
            dry_run: false, // skipped by serde
            ..Config::default()
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.language, original.language);
        assert_eq!(restored.silence_threshold_ms, original.silence_threshold_ms);
        assert_eq!(restored.min_speech_ms, original.min_speech_ms);
        assert_eq!(restored.vad_threshold, original.vad_threshold);
        assert_eq!(restored.log_level, original.log_level);
        assert_eq!(restored.ptt_key, original.ptt_key);
    }

    #[test]
    fn test_deserialization_missing_ptt_key_defaults_to_none() {
        let json = r#"{
            "model_path": "/tmp/model.bin",
            "language": "de",
            "silence_threshold_ms": 800,
            "min_speech_ms": 300,
            "max_buffer_secs": 30.0,
            "vad_threshold": 0.01
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert!(cfg.ptt_key.is_none());
    }

    #[test]
    fn test_deserialization_missing_log_level_defaults_to_info() {
        let json = r#"{
            "model_path": "/tmp/model.bin",
            "language": "de",
            "silence_threshold_ms": 800,
            "min_speech_ms": 300,
            "max_buffer_secs": 30.0,
            "vad_threshold": 0.01
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.log_level, "info");
    }

    #[test]
    fn test_dry_run_is_skipped_in_serialization() {
        let mut cfg = Config::default();
        cfg.dry_run = true;
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("dry_run"), "dry_run must not appear in JSON");
    }

    // ── File I/O ──────────────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let original = Config {
            language: "fr".to_string(),
            vad_threshold: 0.02,
            ptt_key: Some("KEY_F5".to_string()),
            ..Config::default()
        };

        original.save_to(&path).unwrap();
        let loaded = Config::load_from(&path).unwrap();

        assert_eq!(loaded.language, "fr");
        assert_eq!(loaded.vad_threshold, 0.02);
        assert_eq!(loaded.ptt_key, Some("KEY_F5".to_string()));
    }
}
