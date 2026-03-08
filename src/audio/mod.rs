use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, StreamConfig};
use crossbeam_channel::Sender;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tracing::{debug, info, warn};

use crate::config::Config;

mod vad;
use vad::{Vad, VadEvent};

mod webrtc_vad;
pub use webrtc_vad::WebrtcVadFilter;

const WHISPER_SAMPLE_RATE: u32 = 16000;

pub struct AudioCapture {
    device: Device,
    config: Config,
}

impl AudioCapture {
    pub fn new(config: &Config) -> Result<Self> {
        let host = cpal::default_host();

        let device = if let Some(name) = &config.device_name {
            host.input_devices()?
                .find(|d| d.name().map(|n| n.contains(name.as_str())).unwrap_or(false))
                .with_context(|| format!("Audio device '{}' not found", name))?
        } else {
            host.default_input_device()
                .context("No default input device found")?
        };

        info!("Audio device: {}", device.name().unwrap_or_default());
        Ok(Self {
            device,
            config: config.clone(),
        })
    }

    pub fn run(
        &self,
        audio_tx: Sender<Vec<f32>>,
        running: Arc<AtomicBool>,
        ptt_active: Option<Arc<AtomicBool>>,
    ) -> Result<()> {
        let stream_config = self.best_config()?;
        let actual_rate = stream_config.sample_rate.0;
        info!(
            "Stream: {}Hz, {} ch, {:?}",
            actual_rate, stream_config.channels, stream_config.buffer_size
        );

        if ptt_active.is_some() {
            info!("Audio mode: PTT (hold key to record)");
        } else {
            info!("Audio mode: VAD (automatic speech detection)");
        }

        let vad_cfg = self.config.clone();

        // VAD is created even in PTT mode (cheap, kept for potential future hybrid mode)
        let vad = Arc::new(Mutex::new(Vad::new(
            vad_cfg.vad_threshold,
            WHISPER_SAMPLE_RATE,
            vad_cfg.silence_threshold_ms,
            vad_cfg.min_speech_ms,
        )));

        let channels = stream_config.channels as usize;
        let needs_resample = actual_rate != WHISPER_SAMPLE_RATE;
        let resample_ratio = WHISPER_SAMPLE_RATE as f64 / actual_rate as f64;

        let segment: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let max_samples = (vad_cfg.max_buffer_secs * WHISPER_SAMPLE_RATE as f32) as usize;
        let err_fn = |err| warn!("Audio stream error: {}", err);

        let stream = match self.device.default_input_config()?.sample_format() {
            SampleFormat::F32 => {
                let segment_w = segment.clone();
                let vad_clone = vad.clone();
                let audio_tx_inner = audio_tx.clone();
                let ptt = ptt_active.clone();
                let mut ptt_was_active = false;
                let mut wrtc = WebrtcVadFilter::new(vad_cfg.webrtc_vad_aggressiveness);

                self.device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        let resampled =
                            prepare_samples(data, channels, needs_resample, resample_ratio);
                        dispatch(
                            &resampled,
                            &segment_w,
                            &vad_clone,
                            Some(&mut wrtc),
                            ptt.as_ref(),
                            &mut ptt_was_active,
                            &audio_tx_inner,
                            max_samples,
                        );
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::I16 => {
                let segment_w = segment.clone();
                let vad_clone = vad.clone();
                let audio_tx_inner = audio_tx.clone();
                let ptt = ptt_active.clone();
                let mut ptt_was_active = false;
                let mut wrtc2 = WebrtcVadFilter::new(vad_cfg.webrtc_vad_aggressiveness);

                self.device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        let resampled =
                            prepare_samples(&f32_data, channels, needs_resample, resample_ratio);
                        dispatch(
                            &resampled,
                            &segment_w,
                            &vad_clone,
                            Some(&mut wrtc2),
                            ptt.as_ref(),
                            &mut ptt_was_active,
                            &audio_tx_inner,
                            max_samples,
                        );
                    },
                    err_fn,
                    None,
                )?
            }
            fmt => anyhow::bail!("Unsupported sample format: {:?}", fmt),
        };

        stream.play()?;
        if ptt_active.is_some() {
            info!("Ready. Hold the PTT key to record. Ctrl+C to quit.");
        } else {
            info!("Recording started. Speak now...");
        }

        while running.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(())
    }

    fn best_config(&self) -> Result<StreamConfig> {
        let desired = StreamConfig {
            channels: 1,
            sample_rate: SampleRate(WHISPER_SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        if let Ok(supported) = self.device.default_input_config() {
            Ok(supported.config())
        } else {
            Ok(desired)
        }
    }
}

/// Downmix to mono and optionally resample to 16kHz.
fn prepare_samples(
    data: &[f32],
    channels: usize,
    needs_resample: bool,
    resample_ratio: f64,
) -> Vec<f32> {
    let mono: Vec<f32> = if channels == 1 {
        data.to_vec()
    } else {
        data.chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    if needs_resample {
        linear_resample(&mono, resample_ratio)
    } else {
        mono
    }
}

/// Route audio to VAD or PTT handler based on mode.
#[allow(clippy::too_many_arguments)]
fn dispatch(
    resampled: &[f32],
    segment: &Arc<Mutex<Vec<f32>>>,
    vad: &Arc<Mutex<Vad>>,
    webrtc_filter: Option<&mut WebrtcVadFilter>,
    ptt_active: Option<&Arc<AtomicBool>>,
    ptt_was_active: &mut bool,
    tx: &Sender<Vec<f32>>,
    max_samples: usize,
) {
    if let Some(ptt) = ptt_active {
        handle_audio_ptt(resampled, segment, ptt, ptt_was_active, tx, max_samples);
    } else {
        handle_audio_vad(resampled, segment, vad, webrtc_filter, tx, max_samples);
    }
}

/// VAD mode: detect speech automatically, send segment on silence.
fn handle_audio_vad(
    resampled: &[f32],
    segment: &Arc<Mutex<Vec<f32>>>,
    vad: &Arc<Mutex<Vad>>,
    webrtc_filter: Option<&mut WebrtcVadFilter>,
    tx: &Sender<Vec<f32>>,
    max_samples: usize,
) {
    let event = vad.lock().unwrap().process(resampled);
    let mut seg = segment.lock().unwrap();

    match event {
        VadEvent::SpeechStart | VadEvent::None => {
            if vad.lock().unwrap().is_speaking {
                let passes_webrtc = webrtc_filter
                    .map(|f| f.is_speech(resampled))
                    .unwrap_or(true);
                if passes_webrtc {
                    seg.extend_from_slice(resampled);
                    if seg.len() > max_samples {
                        let drain_to = seg.len() - max_samples;
                        seg.drain(..drain_to);
                    }
                }
            }
        }
        VadEvent::SpeechEnd => {
            seg.extend_from_slice(resampled);
            let samples = seg.clone();
            seg.clear();
            debug!(
                "VAD segment: {} samples ({:.1}s)",
                samples.len(),
                samples.len() as f32 / 16000.0
            );
            let _ = tx.send(samples);
        }
        VadEvent::SpeechTooShort => {
            seg.clear();
            debug!("Speech too short, discarding");
        }
    }
}

/// PTT mode: accumulate audio while key held, send buffer on key release.
fn handle_audio_ptt(
    resampled: &[f32],
    segment: &Arc<Mutex<Vec<f32>>>,
    ptt_active: &Arc<AtomicBool>,
    ptt_was_active: &mut bool,
    tx: &Sender<Vec<f32>>,
    max_samples: usize,
) {
    let active = ptt_active.load(Ordering::SeqCst);

    if active {
        let mut seg = segment.lock().unwrap();
        seg.extend_from_slice(resampled);
        // Hard cap to prevent unbounded growth on very long presses
        if seg.len() > max_samples {
            let drain_to = seg.len() - max_samples;
            seg.drain(..drain_to);
        }
    } else if *ptt_was_active {
        // Key just released — flush the accumulated buffer for transcription
        let mut seg = segment.lock().unwrap();
        if !seg.is_empty() {
            let samples = std::mem::take(&mut *seg);
            debug!(
                "PTT segment: {} samples ({:.1}s)",
                samples.len(),
                samples.len() as f32 / 16000.0
            );
            let _ = tx.send(samples);
        }
    }

    *ptt_was_active = active;
}

/// Linear interpolation resampling
fn linear_resample(samples: &[f32], ratio: f64) -> Vec<f32> {
    if samples.is_empty() {
        return vec![];
    }
    let new_len = ((samples.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;
        let s0 = samples[src_idx.min(samples.len() - 1)];
        let s1 = samples[(src_idx + 1).min(samples.len() - 1)];
        out.push(s0 + (s1 - s0) * frac as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    // ── prepare_samples ──────────────────────────────────────────────────────

    #[test]
    fn test_prepare_samples_mono_no_resample() {
        let data: Vec<f32> = (0..8).map(|i| i as f32 * 0.1).collect();
        let result = prepare_samples(&data, 1, false, 1.0);
        assert_eq!(result, data);
    }

    #[test]
    fn test_prepare_samples_stereo_downmix() {
        // Two channels: L=1.0, R=0.0 → mono should be 0.5 per frame
        let stereo: Vec<f32> = (0..8).map(|i| if i % 2 == 0 { 1.0 } else { 0.0 }).collect();
        let result = prepare_samples(&stereo, 2, false, 1.0);
        assert_eq!(result.len(), 4);
        for &s in &result {
            assert!((s - 0.5).abs() < 1e-6, "expected 0.5, got {}", s);
        }
    }

    #[test]
    fn test_prepare_samples_stereo_equal_channels() {
        // L=R=0.4 → mono = 0.4
        let stereo: Vec<f32> = vec![0.4, 0.4, 0.4, 0.4];
        let result = prepare_samples(&stereo, 2, false, 1.0);
        assert_eq!(result.len(), 2);
        for &s in &result {
            assert!((s - 0.4).abs() < 1e-6);
        }
    }

    // ── linear_resample ───────────────────────────────────────────────────────

    #[test]
    fn test_linear_resample_empty() {
        assert!(linear_resample(&[], 2.0).is_empty());
    }

    #[test]
    fn test_linear_resample_ratio_one() {
        let data = vec![0.1, 0.2, 0.3, 0.4];
        let result = linear_resample(&data, 1.0);
        assert_eq!(result.len(), data.len());
        for (a, b) in result.iter().zip(data.iter()) {
            assert!((a - b).abs() < 1e-5, "a={} b={}", a, b);
        }
    }

    #[test]
    fn test_linear_resample_upsample_doubles_length() {
        // ratio 2.0 should approximately double the sample count
        let data: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let result = linear_resample(&data, 2.0);
        // new_len = round(100 * 2.0) = 200
        assert_eq!(result.len(), 200);
    }

    #[test]
    fn test_linear_resample_downsample_halves_length() {
        let data: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let result = linear_resample(&data, 0.5);
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn test_linear_resample_single_sample() {
        let data = vec![0.7f32];
        let result = linear_resample(&data, 2.0);
        // new_len = round(1 * 2.0) = 2; both samples clamp to data[0]
        assert_eq!(result.len(), 2);
        for &s in &result {
            assert!((s - 0.7).abs() < 1e-6);
        }
    }

    // ── handle_audio_ptt ─────────────────────────────────────────────────────

    fn make_segment() -> Arc<Mutex<Vec<f32>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    #[test]
    fn test_ptt_accumulates_while_held() {
        let segment = make_segment();
        let ptt_active = Arc::new(AtomicBool::new(true));
        let mut ptt_was_active = false;
        let (tx, rx) = unbounded();
        let samples = vec![0.1f32; 400];

        handle_audio_ptt(
            &samples,
            &segment,
            &ptt_active,
            &mut ptt_was_active,
            &tx,
            16000,
        );
        assert_eq!(segment.lock().unwrap().len(), 400);
        assert!(rx.try_recv().is_err(), "no segment should be sent yet");
    }

    #[test]
    fn test_ptt_flushes_on_release() {
        let segment = make_segment();
        let ptt_active = Arc::new(AtomicBool::new(true));
        let mut ptt_was_active = false;
        let (tx, rx) = unbounded();
        let samples = vec![0.2f32; 800];

        // Key held — accumulate
        handle_audio_ptt(
            &samples,
            &segment,
            &ptt_active,
            &mut ptt_was_active,
            &tx,
            16000,
        );
        assert_eq!(ptt_was_active, true);

        // Key released — flush
        ptt_active.store(false, std::sync::atomic::Ordering::SeqCst);
        handle_audio_ptt(&[], &segment, &ptt_active, &mut ptt_was_active, &tx, 16000);

        let sent = rx.try_recv().expect("segment should have been sent");
        assert_eq!(sent.len(), 800);
        assert!(segment.lock().unwrap().is_empty());
    }

    #[test]
    fn test_ptt_no_flush_when_never_held() {
        let segment = make_segment();
        let ptt_active = Arc::new(AtomicBool::new(false));
        let mut ptt_was_active = false;
        let (tx, rx) = unbounded();

        handle_audio_ptt(
            &[0.0f32; 800],
            &segment,
            &ptt_active,
            &mut ptt_was_active,
            &tx,
            16000,
        );
        assert!(rx.try_recv().is_err());
        assert!(segment.lock().unwrap().is_empty());
    }

    #[test]
    fn test_ptt_max_cap_trims_buffer() {
        let segment = make_segment();
        let ptt_active = Arc::new(AtomicBool::new(true));
        let mut ptt_was_active = false;
        let (tx, _rx) = unbounded();
        let max_samples = 500;

        // Feed 800 samples with a cap of 500
        let samples = vec![0.1f32; 800];
        handle_audio_ptt(
            &samples,
            &segment,
            &ptt_active,
            &mut ptt_was_active,
            &tx,
            max_samples,
        );
        assert_eq!(segment.lock().unwrap().len(), max_samples);
    }
}

/// List available audio input devices
pub fn list_devices() -> Result<()> {
    let host = cpal::default_host();
    println!("Available input devices:");
    for (i, device) in host.input_devices()?.enumerate() {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let default = host
            .default_input_device()
            .and_then(|d| d.name().ok())
            .map(|dn| dn == name)
            .unwrap_or(false);
        println!(
            "  [{}] {}{}",
            i,
            name,
            if default { " (default)" } else { "" }
        );
    }
    Ok(())
}
