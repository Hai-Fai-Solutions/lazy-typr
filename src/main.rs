use anyhow::Result;
use clap::Parser;
use crossbeam_channel::{bounded, unbounded};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tracing::{debug, error, info};

use whisper_type::audio::{self, AudioCapture};
use whisper_type::config::{Config, Task};
use whisper_type::ptt;
use whisper_type::transcriber::Transcriber;
use whisper_type::typer::Typer;

#[derive(Parser, Debug)]
#[command(name = "whisper-type")]
#[command(about = "Real-time speech-to-text that types into focused input fields")]
struct Args {
    /// Path to Whisper GGML model file
    #[arg(short, long)]
    model: Option<std::path::PathBuf>,

    /// Audio input device (default: system default)
    #[arg(short, long)]
    device: Option<String>,

    /// Whisper language (e.g. "de", "en", auto-detect if not set)
    #[arg(short, long)]
    language: Option<String>,

    /// Show available audio devices and exit
    #[arg(long)]
    list_devices: bool,

    /// VAD silence threshold in milliseconds
    #[arg(long, default_value = "800")]
    silence_ms: u64,

    /// Push-to-talk mode: hold key to record (e.g. "ctrl+space")
    #[arg(long)]
    ptt_key: Option<String>,

    /// Enable GPU (Vulkan) inference
    #[arg(long)]
    gpu: bool,

    /// GPU device index to use for Vulkan inference (implies --gpu)
    #[arg(long, value_name = "N")]
    gpu_device: Option<u32>,

    /// List available Vulkan GPU devices and exit
    #[arg(long)]
    list_gpu_devices: bool,

    /// WebRTC VAD aggressiveness level 0-3 (higher = more noise rejection)
    #[arg(long, value_name = "0-3")]
    webrtc_vad_aggressiveness: Option<u8>,

    /// Whisper inference task: transcribe (default) or translate
    #[arg(long, value_enum)]
    whisper_task: Option<Task>,

    /// Print transcribed text to stdout instead of typing it
    #[arg(long)]
    dry_run: bool,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long)]
    log_level: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // List devices mode (no logging needed)
    if args.list_devices {
        audio::list_devices()?;
        return Ok(());
    }

    if args.list_gpu_devices {
        let devices = whisper_rs::vulkan::list_devices();
        if devices.is_empty() {
            println!("No Vulkan GPU devices found.");
        } else {
            println!("GPU devices:");
            for dev in &devices {
                println!(
                    "  {}: {}  ({} MB total, {} MB free)",
                    dev.id,
                    dev.name,
                    dev.vram.total / 1024 / 1024,
                    dev.vram.free / 1024 / 1024,
                );
            }
        }
        return Ok(());
    }

    // Load config (merge with CLI args)
    let mut config = Config::load_or_default_quiet()?;
    if let Some(model) = args.model {
        config.model_path = model;
    }
    if let Some(device) = args.device {
        config.device_name = Some(device);
    }
    config.apply_language_override(args.language);
    config.apply_whisper_task_override(args.whisper_task);
    config.silence_threshold_ms = args.silence_ms;
    if args.gpu {
        config.use_gpu = true;
    }
    if let Some(dev) = args.gpu_device {
        config.use_gpu = true;
        config.gpu_device = dev;
    }
    config.dry_run = args.dry_run;
    if let Some(level) = args.log_level {
        config.log_level = level;
    }
    if let Some(key) = args.ptt_key {
        config.ptt_key = Some(key);
    }
    if let Some(level) = args.webrtc_vad_aggressiveness {
        if level > 3 {
            eprintln!(
                "Error: webrtc_vad_aggressiveness must be 0-3, got {}",
                level
            );
            std::process::exit(1);
        }
        config.webrtc_vad_aggressiveness = level;
    }

    // Initialize logging with the merged level
    // RUST_LOG env var still overrides everything (via from_default_env)
    let directive = format!("whisper_type={}", config.log_level);
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(directive.parse()?),
        )
        .init();

    let cfg_path = Config::config_path();
    if cfg_path.exists() {
        info!("Loaded config from {}", cfg_path.display());
    }
    info!("Log level: {}", config.log_level);

    // Validate model path
    if !config.model_path.exists() {
        error!(
            "Whisper model not found at: {}\n\
             Download a model with:\n\
             wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \\\n\
             -O ~/.local/share/whisper-type/ggml-base.bin",
            config.model_path.display()
        );
        std::process::exit(1);
    }

    info!("whisper-type starting...");
    info!("Model: {}", config.model_path.display());
    info!("Language: {}", config.language);
    info!(
        "Whisper task: {}",
        match config.whisper_task {
            Task::Transcribe => "transcribe",
            Task::Translate => "translate",
        }
    );
    if config.ptt_key.is_none() {
        info!("Silence threshold: {}ms", config.silence_threshold_ms);
    }
    if config.dry_run {
        info!("Dry-run mode: text will be printed to stdout");
    }
    if config.use_gpu {
        info!(
            "GPU inference: enabled (Vulkan, device {})",
            config.gpu_device
        );
    } else {
        info!("GPU inference: disabled (CPU)");
    }
    info!(
        "WebRTC VAD aggressiveness: {} ({})",
        config.webrtc_vad_aggressiveness,
        match config.webrtc_vad_aggressiveness {
            0 => "quality",
            1 => "low-bitrate",
            2 => "aggressive",
            _ => "very-aggressive",
        }
    );

    // Shared running flag
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        info!("Shutting down...");
        r.store(false, Ordering::SeqCst);
    })
    .ok();

    // Set up PTT if configured
    let ptt_active: Option<Arc<AtomicBool>> = if let Some(ref key_str) = config.ptt_key {
        let key = ptt::parse_key(key_str).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown PTT key: '{}'. Supported keys: {}",
                key_str,
                ptt::supported_keys()
            )
        })?;
        let flag = Arc::new(AtomicBool::new(false));
        ptt::spawn_ptt_monitor(key, flag.clone(), running.clone())?;
        info!("PTT mode enabled: hold '{}' to record", key_str);
        Some(flag)
    } else {
        None
    };

    // Channel: audio chunks (PCM f32 mono 16kHz) → transcriber
    let (audio_tx, audio_rx) = bounded::<Vec<f32>>(32);

    // Channel: transcribed text → typer
    let (text_tx, text_rx) = unbounded::<String>();

    // Start transcriber thread
    let config_t = config.clone();
    let text_tx_t = text_tx.clone();
    let transcriber_handle = std::thread::spawn(move || {
        let mut transcriber = match Transcriber::new(&config_t) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to initialize Whisper: {}", e);
                std::process::exit(1);
            }
        };
        info!("Whisper model loaded ✓");
        while let Ok(samples) = audio_rx.recv() {
            match transcriber.transcribe(&samples) {
                Ok(Some(text)) => {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        debug!("Transcribed: \"{}\"", text);
                        let _ = text_tx_t.send(text);
                    }
                }
                Ok(None) => {}
                Err(e) => error!("Transcription error: {}", e),
            }
        }
    });

    // Start typer thread
    let config_ty = config.clone();
    let typer_handle = std::thread::spawn(move || {
        let typer = Typer::new(config_ty.dry_run);
        while let Ok(text) = text_rx.recv() {
            if let Err(e) = typer.type_text(&text) {
                error!("Failed to type text: {}", e);
            }
        }
    });

    // Start audio capture (blocks until running=false)
    info!("Microphone active. Ctrl+C to quit.");
    let capture = AudioCapture::new(&config)?;
    capture.run(audio_tx, running, ptt_active)?;

    info!("Stopping...");
    drop(capture); // drops CPAL stream → audio_tx drops → audio_rx closes → transcriber exits
    let _ = transcriber_handle.join(); // transcriber exits, drops text_tx_t
    drop(text_tx); // close last text sender so typer can exit
    let _ = typer_handle.join();

    Ok(())
}
