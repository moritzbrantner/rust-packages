use audio_analysis_models::{
    classify_audio, detect_audio_events, diarize_speakers, embed_audio, generate_audio,
    model_catalog, separate_sources, AudioClassificationRequest, AudioEmbeddingRequest,
    AudioEventDetectionRequest, AudioFeatureFrame, AudioFeatureSummary, AudioGenerationRequest,
    AudioModelSelection, FallbackPolicy, SourceSeparationRequest, SpeakerDiarizationRequest,
};
use audio_analysis_recognition as _;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "audio-analysis-recognition-cli",
    version,
    about = "Thin CLI adapter for audio-analysis-recognition"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print package and adapter metadata.
    Info {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Print the generic command schema.
    Schema {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List known audio model presets.
    Models {
        /// Emit only models for a task path segment, for example classify or transcribe.
        #[arg(long)]
        task: Option<String>,
    },
    /// Classify an audio feature summary.
    Classify {
        /// Comma-separated labels for fallback classification.
        #[arg(long)]
        labels: Option<String>,
        /// RMS energy.
        #[arg(long, default_value_t = 0.1)]
        rms: f32,
        /// Peak amplitude.
        #[arg(long, default_value_t = 0.2)]
        peak: f32,
        /// Spectral centroid in Hz.
        #[arg(long, default_value_t = 1_500.0)]
        spectral_centroid_hz: f32,
        /// Model id or preset id.
        #[arg(long)]
        model: Option<String>,
        /// Fallback policy.
        #[arg(long, value_enum, default_value_t = FallbackArg::Heuristic)]
        fallback: FallbackArg,
    },
    /// Detect high-energy events from RMS frames.
    Events {
        /// Comma-separated RMS values, one per synthetic one-second frame.
        #[arg(long)]
        rms: String,
        /// RMS/peak event threshold.
        #[arg(long, default_value_t = 0.2)]
        threshold: f32,
    },
    /// Embed an audio feature summary.
    Embed {
        /// Fallback vector dimensions.
        #[arg(long, default_value_t = 128)]
        dimensions: usize,
        /// RMS energy.
        #[arg(long, default_value_t = 0.1)]
        rms: f32,
        /// Peak amplitude.
        #[arg(long, default_value_t = 0.2)]
        peak: f32,
    },
    /// Create a one-speaker diarization fallback.
    Diarize {
        /// Duration in seconds.
        #[arg(long, default_value_t = 10.0)]
        duration_seconds: f32,
    },
    /// Plan source separation stems.
    Separate {
        /// Comma-separated stem names.
        #[arg(long, default_value = "vocals,drums,bass,other")]
        stems: String,
    },
    /// Validate an audio generation prompt.
    Generate {
        /// Text prompt.
        #[arg(long)]
        prompt: String,
        /// Duration in seconds.
        #[arg(long, default_value_t = 8.0)]
        duration_seconds: f32,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FallbackArg {
    Error,
    Fast,
    Heuristic,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Command::Info { json: false }) {
        Command::Info { json } => print_payload(
            json,
            "audio-analysis-recognition",
            &audio_analysis_recognition_cli::package_metadata_json(),
        ),
        Command::Schema { json } => print_payload(
            json,
            "audio-analysis-recognition command schema",
            &audio_analysis_recognition_cli::command_schema_json(),
        ),
        Command::Models { task } => {
            let task = task.as_deref().and_then(audio_analysis_models::parse_task);
            println!("{}", serde_json::to_string(&model_catalog(task))?);
        }
        Command::Classify {
            labels,
            rms,
            peak,
            spectral_centroid_hz,
            model,
            fallback,
        } => {
            let payload = classify_audio(AudioClassificationRequest {
                source: None,
                labels: split_csv(labels.as_deref()),
                top_k: 3,
                features: Some(AudioFeatureSummary {
                    rms,
                    peak,
                    spectral_centroid_hz,
                    ..AudioFeatureSummary::default()
                }),
                model: selection(model, fallback),
                imported_predictions: Vec::new(),
            })?;
            println!("{}", serde_json::to_string(&payload)?);
        }
        Command::Events { rms, threshold } => {
            let frames = split_csv(Some(&rms))
                .into_iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    let rms = value.parse::<f32>().ok()?;
                    Some(AudioFeatureFrame {
                        start_seconds: index as f32,
                        end_seconds: index as f32 + 1.0,
                        rms,
                        peak: rms,
                    })
                })
                .collect::<Vec<_>>();
            let payload = detect_audio_events(AudioEventDetectionRequest {
                source: None,
                frames,
                threshold,
                model: selection(None, FallbackArg::Heuristic),
                imported_predictions: Vec::new(),
            })?;
            println!("{}", serde_json::to_string(&payload)?);
        }
        Command::Embed {
            dimensions,
            rms,
            peak,
        } => {
            let payload = embed_audio(AudioEmbeddingRequest {
                features: vec![AudioFeatureSummary {
                    rms,
                    peak,
                    ..AudioFeatureSummary::default()
                }],
                dimensions,
                normalize: true,
                model: selection(None, FallbackArg::Fast),
                imported_embeddings: Vec::new(),
            })?;
            println!("{}", serde_json::to_string(&payload)?);
        }
        Command::Diarize { duration_seconds } => {
            let payload = diarize_speakers(SpeakerDiarizationRequest {
                source: None,
                duration_seconds: Some(duration_seconds),
                model: selection(None, FallbackArg::Heuristic),
                imported_segments: Vec::new(),
            })?;
            println!("{}", serde_json::to_string(&payload)?);
        }
        Command::Separate { stems } => {
            let payload = separate_sources(SourceSeparationRequest {
                source: None,
                stems: split_csv(Some(&stems)),
                model: selection(None, FallbackArg::Heuristic),
                imported_stems: Vec::new(),
            })?;
            println!("{}", serde_json::to_string(&payload)?);
        }
        Command::Generate {
            prompt,
            duration_seconds,
        } => {
            let payload = generate_audio(AudioGenerationRequest {
                prompt,
                duration_seconds,
                model: AudioModelSelection::default(),
            })?;
            println!("{}", serde_json::to_string(&payload)?);
        }
    }
    Ok(())
}

fn print_payload(json: bool, title: &str, payload: &str) {
    if json {
        println!("{payload}");
    } else {
        println!("{title}");
        println!("{payload}");
    }
}

fn selection(model_id: Option<String>, fallback: FallbackArg) -> AudioModelSelection {
    AudioModelSelection {
        model_id,
        runtime: None,
        fallback_policy: match fallback {
            FallbackArg::Error => FallbackPolicy::Error,
            FallbackArg::Fast => FallbackPolicy::FastFallback,
            FallbackArg::Heuristic => FallbackPolicy::HeuristicFallback,
        },
    }
}

fn split_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}
