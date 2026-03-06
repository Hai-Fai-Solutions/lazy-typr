/// Simple energy-based Voice Activity Detection
pub struct Vad {
    threshold: f32,
    silence_threshold_samples: usize,
    min_speech_samples: usize,

    // State
    silence_counter: usize,
    speech_counter: usize,
    pub is_speaking: bool,
}

impl Vad {
    pub fn new(
        threshold: f32,
        sample_rate: u32,
        silence_threshold_ms: u64,
        min_speech_ms: u64,
    ) -> Self {
        let silence_threshold_samples = (sample_rate as u64 * silence_threshold_ms / 1000) as usize;
        let min_speech_samples = (sample_rate as u64 * min_speech_ms / 1000) as usize;

        Self {
            threshold,
            silence_threshold_samples,
            min_speech_samples,
            silence_counter: 0,
            speech_counter: 0,
            is_speaking: false,
        }
    }

    /// Process a chunk of samples. Returns true if a speech segment just ended
    /// and samples should be sent for transcription.
    pub fn process(&mut self, samples: &[f32]) -> VadEvent {
        let energy = rms_energy(samples);
        let is_voice = energy > self.threshold;

        if is_voice {
            self.silence_counter = 0;
            self.speech_counter += samples.len();
            if !self.is_speaking {
                self.is_speaking = true;
                return VadEvent::SpeechStart;
            }
        } else {
            if self.is_speaking {
                self.silence_counter += samples.len();
                if self.silence_counter >= self.silence_threshold_samples {
                    self.is_speaking = false;
                    let had_enough_speech = self.speech_counter >= self.min_speech_samples;
                    self.speech_counter = 0;
                    self.silence_counter = 0;
                    if had_enough_speech {
                        return VadEvent::SpeechEnd;
                    } else {
                        return VadEvent::SpeechTooShort;
                    }
                }
            }
        }

        VadEvent::None
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.silence_counter = 0;
        self.speech_counter = 0;
        self.is_speaking = false;
    }
}

#[derive(Debug, PartialEq)]
pub enum VadEvent {
    None,
    SpeechStart,
    SpeechEnd,
    SpeechTooShort,
}

fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_speech(n: usize) -> Vec<f32> {
        (0..n).map(|i| (i as f32 * 0.1).sin() * 0.5).collect()
    }

    fn make_silence(n: usize) -> Vec<f32> {
        vec![0.0f32; n]
    }

    // ── rms_energy ────────────────────────────────────────────────────────────

    #[test]
    fn test_rms_energy_empty() {
        assert_eq!(rms_energy(&[]), 0.0);
    }

    #[test]
    fn test_rms_energy_constant() {
        // RMS of a constant 0.5 signal = 0.5
        let samples = vec![0.5f32; 1000];
        let e = rms_energy(&samples);
        assert!((e - 0.5).abs() < 1e-5, "expected ~0.5, got {}", e);
    }

    #[test]
    fn test_rms_energy_zero() {
        let samples = vec![0.0f32; 500];
        assert_eq!(rms_energy(&samples), 0.0);
    }

    // ── VAD basic ─────────────────────────────────────────────────────────────

    #[test]
    fn test_vad_silence() {
        let mut vad = Vad::new(0.01, 16000, 500, 200);
        let silence = make_silence(1600); // 100 ms silence
        assert_eq!(vad.process(&silence), VadEvent::None);
        assert!(!vad.is_speaking);
    }

    #[test]
    fn test_vad_speech_detection() {
        let mut vad = Vad::new(0.01, 16000, 500, 200);
        let speech = make_speech(3200); // 200 ms of speech
        let event = vad.process(&speech);
        assert_eq!(event, VadEvent::SpeechStart);
        assert!(vad.is_speaking);
    }

    // ── VAD speech-end ────────────────────────────────────────────────────────

    #[test]
    fn test_vad_speech_end() {
        // threshold=0.01, silence_threshold=500ms (8000 samples), min_speech=200ms (3200)
        let mut vad = Vad::new(0.01, 16000, 500, 200);

        // Start speaking (400 ms = 6400 samples → well above min_speech)
        let speech = make_speech(6400);
        assert_eq!(vad.process(&speech), VadEvent::SpeechStart);
        assert!(vad.is_speaking);

        // Feed silence past the threshold in one chunk (600 ms = 9600 samples)
        let silence = make_silence(9600);
        let event = vad.process(&silence);
        assert_eq!(event, VadEvent::SpeechEnd);
        assert!(!vad.is_speaking);
    }

    #[test]
    fn test_vad_speech_too_short() {
        // min_speech=500ms (8000 samples), silence_threshold=200ms (3200 samples)
        let mut vad = Vad::new(0.01, 16000, 200, 500);

        // Very brief speech (100 ms = 1600 samples → below min_speech)
        let speech = make_speech(1600);
        assert_eq!(vad.process(&speech), VadEvent::SpeechStart);

        // Silence past the threshold
        let silence = make_silence(4000);
        let event = vad.process(&silence);
        assert_eq!(event, VadEvent::SpeechTooShort);
        assert!(!vad.is_speaking);
    }

    // ── VAD reset ────────────────────────────────────────────────────────────

    #[test]
    fn test_vad_reset_clears_state() {
        let mut vad = Vad::new(0.01, 16000, 500, 200);

        let speech = make_speech(3200);
        vad.process(&speech);
        assert!(vad.is_speaking);

        vad.reset();
        assert!(!vad.is_speaking);

        // After reset, silence should return None (not SpeechEnd)
        let silence = make_silence(9600);
        assert_eq!(vad.process(&silence), VadEvent::None);
    }

    // ── VAD multiple cycles ────────────────────────────────────────────────────

    #[test]
    fn test_vad_multiple_speech_cycles() {
        let mut vad = Vad::new(0.01, 16000, 200, 100);
        // silence_threshold=200ms (3200 samples), min_speech=100ms (1600 samples)

        let speech = make_speech(3200); // 200 ms
        let silence = make_silence(4000); // 250 ms — past threshold

        // First cycle
        assert_eq!(vad.process(&speech), VadEvent::SpeechStart);
        assert_eq!(vad.process(&silence), VadEvent::SpeechEnd);

        // Second cycle — VAD must be usable again
        assert_eq!(vad.process(&speech), VadEvent::SpeechStart);
        assert_eq!(vad.process(&silence), VadEvent::SpeechEnd);
    }

    // ── VAD silence while not speaking ───────────────────────────────────────

    #[test]
    fn test_vad_silence_while_not_speaking_is_none() {
        let mut vad = Vad::new(0.01, 16000, 500, 200);
        // Feed lots of silence without ever speaking — should always be None
        for _ in 0..5 {
            let silence = make_silence(8000);
            assert_eq!(vad.process(&silence), VadEvent::None);
        }
    }
}
