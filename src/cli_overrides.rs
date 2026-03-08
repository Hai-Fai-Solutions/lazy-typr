use crate::config::Config;

/// Runtime CLI overrides that should be merged on top of config file values.
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub language: Option<String>,
}

pub fn apply_cli_overrides(config: &mut Config, overrides: &CliOverrides) {
    if let Some(language) = overrides.language.as_ref() {
        config.language = language.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_cli_overrides, CliOverrides};
    use crate::config::Config;

    #[test]
    fn no_cli_language_keeps_existing_config_language() {
        let mut cfg = Config {
            language: "fr".to_string(),
            ..Config::default()
        };
        let overrides = CliOverrides { language: None };

        apply_cli_overrides(&mut cfg, &overrides);

        assert_eq!(cfg.language, "fr");
    }

    #[test]
    fn cli_language_overrides_existing_config_language() {
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
}
