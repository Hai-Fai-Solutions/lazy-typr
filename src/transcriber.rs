use anyhow::{Context, Result};
use tracing::debug;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::config::{Config, Task};

pub struct Transcriber {
    ctx: WhisperContext,
    language: String,
    task: Task,
}

impl Transcriber {
    pub fn new(config: &Config) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            config.model_path.to_str().context("Invalid model path")?,
            WhisperContextParameters {
                use_gpu: false, // TODO Task 6: replace with resolved backend
                ..Default::default()
            },
        )
        .context("Failed to load Whisper model")?;

        Ok(Self {
            ctx,
            language: config.language.clone(),
            task: config.whisper_task.clone(),
        })
    }

    /// Transcribe a 16kHz mono f32 PCM buffer.
    /// Returns Some(text) if speech was detected, None otherwise.
    pub fn transcribe(&mut self, samples: &[f32]) -> Result<Option<String>> {
        if samples.is_empty() {
            return Ok(None);
        }

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Language
        if self.language == "auto" {
            params.set_language(None);
        } else {
            params.set_language(Some(&self.language));
        }

        // Task: explicitly set on every inference to prevent silent drift
        params.set_translate(matches!(self.task, Task::Translate));

        // Performance settings
        params.set_n_threads(num_cpus());
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(false);
        params.set_no_context(true); // Don't use context from previous segments
        params.set_suppress_blank(true);

        // Token timestamps for filtering
        params.set_token_timestamps(false);

        let mut state = self
            .ctx
            .create_state()
            .context("Failed to create Whisper state")?;

        state
            .full(params, samples)
            .map_err(|e| anyhow::anyhow!("Whisper inference failed: {:?}", e))?;

        let n_segments = state.full_n_segments();
        debug!("Whisper produced {} segments", n_segments);

        if n_segments == 0 {
            return Ok(None);
        }

        let mut result = String::new();
        for i in 0..n_segments {
            let segment = state
                .get_segment(i)
                .ok_or_else(|| anyhow::anyhow!("Failed to get segment {}", i))?;
            let seg_text = segment
                .to_str_lossy()
                .map_err(|e| anyhow::anyhow!("Failed to get segment text: {:?}", e))?;
            let seg = seg_text.trim();

            // Filter out common hallucinations / noise markers
            if is_hallucination(seg) {
                debug!("Filtered hallucination: {:?}", seg);
                continue;
            }

            if !result.is_empty() {
                result.push(' ');
            }
            result.push_str(seg);
        }

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}

/// Whisper sometimes hallucinates these patterns on silence/noise
fn is_hallucination(text: &str) -> bool {
    let lower = text.to_lowercase();
    let hallucinations = [
        "[musik]",
        "[music]",
        "(musik)",
        "(music)",
        "[laughter]",
        "[applause]",
        "[silence]",
        "[ musik ]",
        "[ music ]",
        "vielen dank",
        "danke schön",
        "thank you for watching",
        "untertitel",
        "subtitles",
        "www.",
        ".com",
        "♪",
        "♫",
    ];
    hallucinations.iter().any(|h| lower.contains(h))
        || lower.chars().all(|c| c == '.' || c == ' ' || c == '\n')
}

fn num_cpus() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
        .min(8) // Whisper benefits from up to ~8 threads
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_hallucination ──────────────────────────────────────────────────────

    #[test]
    fn test_hallucination_musik_tag() {
        assert!(is_hallucination("[Musik]"));
        assert!(is_hallucination("[musik]"));
        assert!(is_hallucination("[ Musik ]"));
    }

    #[test]
    fn test_hallucination_music_tag() {
        assert!(is_hallucination("[Music]"));
        assert!(is_hallucination("[music]"));
        assert!(is_hallucination("(Music)"));
        assert!(is_hallucination("(music)"));
    }

    #[test]
    fn test_hallucination_other_markers() {
        assert!(is_hallucination("[laughter]"));
        assert!(is_hallucination("[applause]"));
        assert!(is_hallucination("[silence]"));
    }

    #[test]
    fn test_hallucination_music_symbols() {
        assert!(is_hallucination("♪"));
        assert!(is_hallucination("♫"));
    }

    #[test]
    fn test_hallucination_filler_phrases() {
        assert!(is_hallucination("Vielen Dank"));
        assert!(is_hallucination("Danke schön"));
        assert!(is_hallucination("Thank you for watching"));
        assert!(is_hallucination("Untertitel by XY"));
        assert!(is_hallucination("Subtitles"));
    }

    #[test]
    fn test_hallucination_url_fragments() {
        assert!(is_hallucination("www.example.com"));
        assert!(is_hallucination("visit example.com"));
    }

    #[test]
    fn test_hallucination_dots_only() {
        assert!(is_hallucination("..."));
        assert!(is_hallucination(". . ."));
        assert!(is_hallucination("   "));
    }

    #[test]
    fn test_hallucination_empty_string() {
        // vacuously true: all chars (none) satisfy the predicate
        assert!(is_hallucination(""));
    }

    #[test]
    fn test_not_hallucination_german() {
        assert!(!is_hallucination("Guten Morgen"));
        assert!(!is_hallucination("Ich brauche Hilfe."));
        assert!(!is_hallucination("Das ist ein Test."));
    }

    #[test]
    fn test_not_hallucination_english() {
        assert!(!is_hallucination("Hello world"));
        assert!(!is_hallucination("The quick brown fox."));
        assert!(!is_hallucination("How are you?"));
    }

    // ── num_cpus ──────────────────────────────────────────────────────────────

    #[test]
    fn test_num_cpus_in_valid_range() {
        let n = num_cpus();
        assert!(n >= 1, "num_cpus should be at least 1, got {}", n);
        assert!(n <= 8, "num_cpus should be capped at 8, got {}", n);
    }
}
