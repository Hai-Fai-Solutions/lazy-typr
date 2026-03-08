/// Integration tests for the VAD → channel pipeline.
///
/// These tests simulate real audio chunks flowing through the VAD state machine
/// and verify that the expected segment events arrive on the channel.
use crossbeam_channel::unbounded;

// We reproduce the minimal VAD logic here to avoid depending on private items.
// The real tests in audio/vad.rs cover the implementation; here we test the
// observable behaviour at the boundary (segments arriving on a channel) using
// a thin wrapper that mimics what AudioCapture does internally.

/// A minimal stand-in that drives VAD and forwards segments via a channel,
/// mirroring the logic in `handle_audio_vad`.
///
///

#[allow(dead_code)]
struct VadPipeline {
    threshold: f32,
    sample_rate: u32,
    silence_samples: usize,
    min_speech_samples: usize,
    silence_counter: usize,
    speech_counter: usize,
    is_speaking: bool,
    buffer: Vec<f32>,
    tx: crossbeam_channel::Sender<Vec<f32>>,
}

impl VadPipeline {
    fn new(
        threshold: f32,
        sample_rate: u32,
        silence_ms: u64,
        min_speech_ms: u64,
        tx: crossbeam_channel::Sender<Vec<f32>>,
    ) -> Self {
        Self {
            threshold,
            sample_rate,
            silence_samples: (sample_rate as u64 * silence_ms / 1000) as usize,
            min_speech_samples: (sample_rate as u64 * min_speech_ms / 1000) as usize,
            silence_counter: 0,
            speech_counter: 0,
            is_speaking: false,
            buffer: Vec::new(),
            tx,
        }
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let s: f32 = samples.iter().map(|x| x * x).sum();
        (s / samples.len() as f32).sqrt()
    }

    fn push(&mut self, samples: &[f32]) {
        let is_voice = Self::rms(samples) > self.threshold;

        if is_voice {
            self.silence_counter = 0;
            self.speech_counter += samples.len();
            self.is_speaking = true;
            self.buffer.extend_from_slice(samples);
        } else if self.is_speaking {
            self.silence_counter += samples.len();
            self.buffer.extend_from_slice(samples);
            if self.silence_counter >= self.silence_samples {
                let had_enough = self.speech_counter >= self.min_speech_samples;
                self.is_speaking = false;
                let buf = std::mem::take(&mut self.buffer);
                self.speech_counter = 0;
                self.silence_counter = 0;
                if had_enough {
                    let _ = self.tx.send(buf);
                }
            }
        }
    }
}

fn speech_chunk(n: usize) -> Vec<f32> {
    (0..n).map(|i| (i as f32 * 0.1).sin() * 0.5).collect()
}

fn silence_chunk(n: usize) -> Vec<f32> {
    vec![0.0f32; n]
}

#[test]
fn single_speech_segment_arrives_on_channel() {
    let (tx, rx) = unbounded();
    let mut pipeline = VadPipeline::new(0.01, 16000, 300, 100, tx);

    // 300 ms speech
    pipeline.push(&speech_chunk(4800));
    // 400 ms silence (past the 300 ms threshold)
    pipeline.push(&silence_chunk(6400));

    let segment = rx.try_recv().expect("one segment should arrive");
    assert!(!segment.is_empty());
}

#[test]
fn two_speech_segments_produce_two_channel_messages() {
    let (tx, rx) = unbounded();
    let mut pipeline = VadPipeline::new(0.01, 16000, 200, 100, tx);

    // First utterance
    pipeline.push(&speech_chunk(3200)); // 200 ms speech
    pipeline.push(&silence_chunk(4000)); // 250 ms silence

    // Second utterance
    pipeline.push(&speech_chunk(3200));
    pipeline.push(&silence_chunk(4000));

    assert!(rx.try_recv().is_ok(), "first segment");
    assert!(rx.try_recv().is_ok(), "second segment");
    assert!(rx.try_recv().is_err(), "no third segment");
}

#[test]
fn too_short_speech_does_not_produce_segment() {
    let (tx, rx) = unbounded();
    // min_speech = 500 ms = 8000 samples; silence_threshold = 200 ms = 3200 samples
    let mut pipeline = VadPipeline::new(0.01, 16000, 200, 500, tx);

    // Only 100 ms of speech (below min_speech)
    pipeline.push(&speech_chunk(1600));
    pipeline.push(&silence_chunk(4000));

    assert!(
        rx.try_recv().is_err(),
        "short speech should not produce a segment"
    );
}

#[test]
fn continuous_silence_produces_no_segments() {
    let (tx, rx) = unbounded();
    let mut pipeline = VadPipeline::new(0.01, 16000, 300, 100, tx);

    for _ in 0..10 {
        pipeline.push(&silence_chunk(4800));
    }

    assert!(
        rx.try_recv().is_err(),
        "silence should not produce segments"
    );
}

// ── WebRTC VAD two-stage gate ─────────────────────────────────────────────

#[test]
fn webrtc_vad_rejects_silence() {
    use whisper_type::audio::WebrtcVadFilter;
    let mut filter = WebrtcVadFilter::new(2);
    let silence = vec![0.0f32; 16000]; // 1 second
    assert!(
        !filter.is_speech(&silence),
        "WebRTC VAD must not classify silence as speech"
    );
}

#[test]
fn webrtc_vad_accepts_valid_aggressiveness_range() {
    use whisper_type::audio::WebrtcVadFilter;
    for level in 0u8..=3 {
        let mut filter = WebrtcVadFilter::new(level);
        let silence = vec![0.0f32; 160];
        let _ = filter.is_speech(&silence);
    }
}
