use webrtc_vad::{SampleRate, Vad, VadMode};

const FRAME_SAMPLES: usize = 160; // 10ms @ 16kHz

pub struct WebrtcVadFilter {
    vad: Vad,
    frame_buf: Vec<i16>,
}

impl WebrtcVadFilter {
    pub fn new(aggressiveness: u8) -> Self {
        let mode = match aggressiveness {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            _ => VadMode::VeryAggressive,
        };
        Self {
            vad: Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, mode),
            frame_buf: Vec::with_capacity(FRAME_SAMPLES),
        }
    }

    /// Feed f32 samples (16kHz mono). Returns true if any complete 10ms frame
    /// was classified as speech.
    pub fn is_speech(&mut self, samples: &[f32]) -> bool {
        let i16_samples: Vec<i16> = samples.iter().map(|&s| f32_to_i16(s)).collect();
        self.frame_buf.extend_from_slice(&i16_samples);

        let mut any_speech = false;
        while self.frame_buf.len() >= FRAME_SAMPLES {
            let frame: Vec<i16> = self.frame_buf.drain(..FRAME_SAMPLES).collect();
            if self.vad.is_voice_segment(&frame).unwrap_or(false) {
                any_speech = true;
            }
        }
        any_speech
    }
}

fn f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * 32767.0) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_not_speech() {
        let mut filter = WebrtcVadFilter::new(2);
        let silence = vec![0.0f32; 16000];
        assert!(!filter.is_speech(&silence));
    }

    #[test]
    fn partial_frame_returns_false() {
        let mut filter = WebrtcVadFilter::new(2);
        let partial = vec![0.5f32; 100];
        assert!(!filter.is_speech(&partial));
    }

    #[test]
    fn partial_frame_is_buffered_and_completes_next_call() {
        let mut filter = WebrtcVadFilter::new(2);
        let part_a = vec![0.0f32; 100];
        let part_b = vec![0.0f32; 60];
        assert!(!filter.is_speech(&part_a));
        let _ = filter.is_speech(&part_b);
    }

    #[test]
    fn f32_to_i16_zero() {
        assert_eq!(f32_to_i16(0.0), 0);
    }

    #[test]
    fn f32_to_i16_positive_clamps() {
        assert_eq!(f32_to_i16(1.0), 32767);
        assert_eq!(f32_to_i16(2.0), 32767);
    }

    #[test]
    fn f32_to_i16_negative_clamps() {
        assert_eq!(f32_to_i16(-1.0), -32767);
        assert_eq!(f32_to_i16(-2.0), -32767);
    }

    #[test]
    fn new_accepts_all_aggressiveness_levels() {
        for level in 0u8..=3 {
            let _ = WebrtcVadFilter::new(level);
        }
    }
}
