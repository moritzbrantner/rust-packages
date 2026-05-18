//! Internal module support for audio voice analysis.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use audio_analysis_core::{ChannelMix, FrameSpec};
use audio_analysis_io::{decode_audio_to_mono_f32, AudioInput, AudioInputOptions};
use audio_analysis_pitch::{
    segment_pitch_track, AutocorrelationPitchDetector, PitchDetectorConfig, PitchFrameEstimate,
    PitchSmoother,
};
use audio_analysis_rhythm::{
    detect_onsets, estimate_tempo, onset_envelope, OnsetDetectorConfig, TempoEstimate,
    TempoEstimatorConfig,
};
use audio_analysis_separation::{DemucsModel, HtdemucsOptions, HtdemucsSeparator, Stem};
use serde::{Deserialize, Serialize};
use text_analysis_transcription::WhisperCppProgressEvent;
use video_analysis_core::{DetectError, Result};

use crate::workflow_support::{
    display_path, transcribe_media, validate_local_file, write_json_report,
};
use crate::{
    AudioSeparationConfig, AudioSeparationReport, AudioStemReport, CapabilityReport,
    TranscriptionConfig, TranscriptionEngine, TranscriptionReport, AUDIO_VOICE_ANALYSIS_USE_CASE,
};

const DEFAULT_PITCH_MIN_HZ: f32 = 70.0;
const DEFAULT_PITCH_MAX_HZ: f32 = 1_200.0;
const DEFAULT_PITCH_CONFIDENCE: f32 = 0.7;
const DEFAULT_PITCH_SMOOTHING: usize = 5;
const DEFAULT_FRAME_SIZE: usize = 2_048;
const DEFAULT_HOP_SIZE: usize = 512;
const DEFAULT_NOTE_GAP_SECONDS: f64 = 0.08;
const DEFAULT_NOTE_MIN_DURATION_SECONDS: f64 = 0.12;

#[derive(Debug, Clone)]
/// Data type for audio voice analysis request.
pub struct AudioVoiceAnalysisRequest {
    /// The input value.
    pub input: PathBuf,
    /// The work dir value.
    pub work_dir: PathBuf,
    /// The output value.
    pub output: Option<PathBuf>,
    /// The transcription value.
    pub transcription: TranscriptionConfig,
    /// The audio separation value.
    pub audio_separation: AudioSeparationConfig,
    /// The require voice stem value.
    pub require_voice_stem: bool,
}

impl Default for AudioVoiceAnalysisRequest {
    fn default() -> Self {
        Self {
            input: PathBuf::from("input.wav"),
            work_dir: PathBuf::from("use-case-output/audio-voice-analysis"),
            output: None,
            transcription: TranscriptionConfig {
                enabled: true,
                engine: TranscriptionEngine::default(),
                command: None,
                whisper_cpp: crate::WhisperCppConfig::default(),
            },
            audio_separation: AudioSeparationConfig {
                enabled: true,
                ..AudioSeparationConfig::default()
            },
            require_voice_stem: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio voice analysis run request.
pub struct AudioVoiceAnalysisRunRequest {
    /// The input value.
    pub input: PathBuf,
    /// The work dir value.
    pub work_dir: Option<PathBuf>,
    #[serde(default)]
    /// The transcription value.
    pub transcription: TranscriptionConfig,
    #[serde(default = "default_audio_separation_config")]
    /// The audio separation value.
    pub audio_separation: AudioSeparationConfig,
    #[serde(default)]
    /// The require voice stem value.
    pub require_voice_stem: bool,
}

impl AudioVoiceAnalysisRunRequest {
    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        validate_local_file(&self.input)?;
        self.audio_separation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio source report.
pub struct AudioSourceReport {
    /// The local audio value.
    pub local_audio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio voice analysis asset report.
pub struct AudioVoiceAnalysisAssetReport {
    /// The work dir value.
    pub work_dir: String,
    /// The report path value.
    pub report_path: String,
    /// The transcription audio value.
    pub transcription_audio: Option<String>,
    /// The voice stem value.
    pub voice_stem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for sung note report.
pub struct SungNoteReport {
    /// The start seconds value.
    pub start_seconds: f64,
    /// The end seconds value.
    pub end_seconds: f64,
    /// The frequency hz value.
    pub frequency_hz: f32,
    /// The midi note value.
    pub midi_note: f32,
    /// The note name value.
    pub note_name: String,
    /// Confidence score for this value.
    pub confidence: f32,
    /// The frames value.
    pub frames: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Data type for audio voice analysis report.
pub struct AudioVoiceAnalysisReport {
    #[serde(alias = "use_case")]
    /// The workflow value.
    pub workflow: String,
    /// The source value.
    pub source: AudioSourceReport,
    /// The assets value.
    pub assets: AudioVoiceAnalysisAssetReport,
    /// The capabilities value.
    pub capabilities: CapabilityReport,
    /// The voice source value.
    pub voice_source: String,
    /// The tempo BPM value.
    pub tempo_bpm: Option<f32>,
    /// The tempo confidence value.
    pub tempo_confidence: f32,
    /// The sung notes value.
    pub sung_notes: Vec<SungNoteReport>,
    /// The transcription value.
    pub transcription: TranscriptionReport,
    /// The separation value.
    pub separation: Option<AudioSeparationReport>,
}

/// Runs audio voice analysis.
pub fn run_audio_voice_analysis(
    args: AudioVoiceAnalysisRequest,
) -> Result<AudioVoiceAnalysisReport> {
    validate_local_file(&args.input)?;
    std::fs::create_dir_all(&args.work_dir)?;
    let report_path = args
        .output
        .clone()
        .unwrap_or_else(|| args.work_dir.join("analysis.json"));

    let mut completed = Vec::new();
    let mut skipped = Vec::new();

    let (separation, voice_stem_path) = if args.audio_separation.enabled {
        match run_audio_separation(&args.audio_separation, &args.input, &args.work_dir) {
            Ok(report) => {
                completed.push("audio_separation".to_string());
                let voice = voice_stem_path(&report);
                (Some(report), voice)
            }
            Err(err) => {
                skipped.push(format!("audio separation: {err}"));
                (None, None)
            }
        }
    } else {
        skipped.push("audio separation disabled".to_string());
        (None, None)
    };

    if args.require_voice_stem && voice_stem_path.is_none() {
        return Err(DetectError::Source(
            "voice stem was required but separation did not produce vocals".to_string(),
        ));
    }

    let transcription_input = voice_stem_path
        .as_deref()
        .unwrap_or(args.input.as_path())
        .to_path_buf();
    let mut progress = |_event: WhisperCppProgressEvent| {};
    let (transcription, transcription_audio) = transcribe_media(
        &args.transcription,
        &transcription_input,
        &args.work_dir,
        &mut progress,
    );
    if transcription.status == "completed" {
        completed.push("transcription".to_string());
    } else {
        skipped.push(format!(
            "transcription: {}",
            transcription.message.as_deref().unwrap_or("not available")
        ));
    }

    let tempo = analyze_tempo(&args.input)?;
    completed.push("tempo_estimation".to_string());

    let voice_analysis_path = voice_stem_path
        .as_deref()
        .unwrap_or(args.input.as_path())
        .to_path_buf();
    let sung_notes = analyze_sung_notes(&voice_analysis_path)?;
    completed.push("pitch_tracking".to_string());

    Ok(AudioVoiceAnalysisReport {
        workflow: AUDIO_VOICE_ANALYSIS_USE_CASE.to_string(),
        source: AudioSourceReport {
            local_audio: display_path(&args.input),
        },
        assets: AudioVoiceAnalysisAssetReport {
            work_dir: display_path(&args.work_dir),
            report_path: display_path(&report_path),
            transcription_audio: transcription_audio.as_ref().map(|path| display_path(path)),
            voice_stem: voice_stem_path.as_ref().map(|path| display_path(path)),
        },
        capabilities: CapabilityReport { completed, skipped },
        voice_source: voice_analysis_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("input")
            .to_string(),
        tempo_bpm: tempo.bpm,
        tempo_confidence: tempo.confidence,
        sung_notes,
        transcription,
        separation,
    })
}

/// Runs audio voice analysis workflow.
pub fn run_audio_voice_analysis_workflow(
    request: AudioVoiceAnalysisRunRequest,
    work_dir: PathBuf,
    report_path: PathBuf,
) -> Result<AudioVoiceAnalysisReport> {
    request.validate()?;
    let report = run_audio_voice_analysis(AudioVoiceAnalysisRequest {
        input: request.input,
        work_dir,
        output: Some(report_path.clone()),
        transcription: request.transcription,
        audio_separation: request.audio_separation,
        require_voice_stem: request.require_voice_stem,
    })?;
    write_audio_voice_analysis_report(&report_path, &report)?;
    Ok(report)
}

/// Writes audio voice analysis report.
pub fn write_audio_voice_analysis_report(
    path: &Path,
    report: &AudioVoiceAnalysisReport,
) -> Result<()> {
    write_json_report(path, report)
}

fn default_audio_separation_config() -> AudioSeparationConfig {
    AudioSeparationConfig {
        enabled: true,
        ..AudioSeparationConfig::default()
    }
}

fn run_audio_separation(
    config: &AudioSeparationConfig,
    audio_path: &Path,
    work_dir: &Path,
) -> Result<AudioSeparationReport> {
    let model = config
        .model
        .as_deref()
        .map(DemucsModel::from_str)
        .transpose()?
        .unwrap_or_default();
    let output_dir = config
        .output_dir
        .clone()
        .unwrap_or_else(|| work_dir.join("separated"));
    let mut options = HtdemucsOptions::new(&output_dir).model(model.clone());
    if let Some(command) = &config.command {
        options = options.command(command.command.clone());
        for arg in &command.args {
            options = options.command_arg(arg.clone());
        }
    }
    if let Some(two_stems) = &config.two_stems {
        options = options.two_stems(Stem::from_str(two_stems)?);
    } else {
        options = options.two_stems(Stem::Vocals);
    }
    if let Some(device) = &config.device {
        options = options.device(device.clone());
    }

    let separator = HtdemucsSeparator::new(options)?;
    let result = separator.separate(audio_path)?;
    Ok(AudioSeparationReport {
        status: "completed".to_string(),
        model: result.model.to_string(),
        output_dir: display_path(&result.output_dir),
        stems: result
            .stems
            .into_iter()
            .filter(|stem| stem.exists)
            .map(|stem| AudioStemReport {
                stem: stem.stem.to_string(),
                path: display_path(&stem.path),
                bytes: stem.bytes,
            })
            .collect(),
        message: None,
    })
}

fn voice_stem_path(report: &AudioSeparationReport) -> Option<PathBuf> {
    report
        .stems
        .iter()
        .find(|stem| stem.stem == "vocals")
        .map(|stem| PathBuf::from(stem.path.clone()))
}

fn analyze_tempo(path: &Path) -> Result<TempoEstimate> {
    let (metadata, samples) = decode_audio_to_mono_f32(
        AudioInput::File(path.to_path_buf()),
        AudioInputOptions::recorded(),
        ChannelMix::Average,
    )?;
    let envelope = onset_envelope(
        &samples,
        metadata.sample_rate,
        FrameSpec::new(1024, 256).map_err(|err| DetectError::Source(err.to_string()))?,
    )?;
    let onsets = detect_onsets(&envelope, OnsetDetectorConfig::default())?;
    estimate_tempo(&onsets, TempoEstimatorConfig::default())
}

fn analyze_sung_notes(path: &Path) -> Result<Vec<SungNoteReport>> {
    let (metadata, samples) = decode_audio_to_mono_f32(
        AudioInput::File(path.to_path_buf()),
        AudioInputOptions::recorded(),
        ChannelMix::Average,
    )?;
    if samples.len() < DEFAULT_FRAME_SIZE {
        return Ok(Vec::new());
    }

    let detector = AutocorrelationPitchDetector::new(PitchDetectorConfig {
        min_frequency_hz: DEFAULT_PITCH_MIN_HZ,
        max_frequency_hz: DEFAULT_PITCH_MAX_HZ,
        confidence_threshold: DEFAULT_PITCH_CONFIDENCE,
    })?;
    let mut smoother = PitchSmoother::new(DEFAULT_PITCH_SMOOTHING)?;
    let mut frames = Vec::new();
    for start in (0..=samples.len() - DEFAULT_FRAME_SIZE).step_by(DEFAULT_HOP_SIZE) {
        let frame = &samples[start..start + DEFAULT_FRAME_SIZE];
        let estimate = smoother.smooth(detector.estimate_samples(frame, metadata.sample_rate)?);
        let start_seconds = start as f64 / metadata.sample_rate as f64;
        let end_seconds = (start + DEFAULT_FRAME_SIZE) as f64 / metadata.sample_rate as f64;
        frames.push(PitchFrameEstimate {
            start_seconds,
            end_seconds,
            frequency_hz: estimate.frequency_hz,
            confidence: estimate.confidence,
        });
    }

    Ok(segment_pitch_track(
        &frames,
        DEFAULT_NOTE_GAP_SECONDS,
        DEFAULT_NOTE_MIN_DURATION_SECONDS,
    )
    .into_iter()
    .map(|segment| SungNoteReport {
        start_seconds: segment.start_seconds,
        end_seconds: segment.end_seconds,
        frequency_hz: segment.frequency_hz,
        midi_note: segment.midi_note,
        note_name: segment.note_name,
        confidence: segment.confidence,
        frames: segment.frames,
    })
    .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_analysis_test_support::{click_track, stepped_tones, write_pcm16_wav};

    fn skip_if_ffmpeg_tools_unavailable() -> bool {
        if video_analysis_ffmpeg::is_ffmpeg_available()
            && video_analysis_ffmpeg::is_ffprobe_available()
        {
            return false;
        }
        eprintln!("skipping audio voice analysis test because ffmpeg/ffprobe is unavailable");
        true
    }

    #[test]
    fn audio_voice_analysis_segments_pitch_track_into_notes() {
        if skip_if_ffmpeg_tools_unavailable() {
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.wav");
        let samples = stepped_tones(&[(440.0, 0.25), (493.88, 0.25)], 16_000);
        write_pcm16_wav(&path, 16_000, 1, &samples).unwrap();

        let notes = analyze_sung_notes(&path).unwrap();
        assert!(notes.iter().any(|note| note.note_name == "A4"));
        assert!(notes.iter().any(|note| note.note_name == "B4"));
    }

    #[test]
    fn audio_voice_analysis_reports_tempo_from_click_like_input() {
        if skip_if_ffmpeg_tools_unavailable() {
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tempo.wav");
        let samples = click_track(16_000, 120.0, 4.0);
        write_pcm16_wav(&path, 16_000, 1, &samples).unwrap();

        let tempo = analyze_tempo(&path).unwrap();
        assert!(tempo.bpm.unwrap() > 100.0);
        assert!(tempo.confidence > 0.0);
    }

    #[test]
    fn report_roundtrips_for_each_new_use_case() {
        let report = AudioVoiceAnalysisReport {
            workflow: AUDIO_VOICE_ANALYSIS_USE_CASE.to_string(),
            source: AudioSourceReport {
                local_audio: "input.wav".to_string(),
            },
            assets: AudioVoiceAnalysisAssetReport {
                work_dir: "work".to_string(),
                report_path: "analysis.json".to_string(),
                transcription_audio: Some("audio.wav".to_string()),
                voice_stem: Some("vocals.wav".to_string()),
            },
            capabilities: CapabilityReport {
                completed: vec!["tempo_estimation".to_string()],
                skipped: Vec::new(),
            },
            voice_source: "vocals.wav".to_string(),
            tempo_bpm: Some(120.0),
            tempo_confidence: 0.8,
            sung_notes: vec![SungNoteReport {
                start_seconds: 0.0,
                end_seconds: 0.5,
                frequency_hz: 440.0,
                midi_note: 69.0,
                note_name: "A4".to_string(),
                confidence: 0.9,
                frames: 4,
            }],
            transcription: TranscriptionReport {
                status: "completed".to_string(),
                text: Some("la".to_string()),
                segments: Vec::new(),
                message: None,
            },
            separation: None,
        };

        let value = serde_json::to_vec(&report).unwrap();
        let decoded: AudioVoiceAnalysisReport = serde_json::from_slice(&value).unwrap();
        assert_eq!(decoded, report);
    }
}
