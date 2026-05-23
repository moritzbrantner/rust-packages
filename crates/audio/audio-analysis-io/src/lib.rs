#![doc = include_str!("../README.md")]

pub mod surface;
use std::path::{Path, PathBuf};

use audio_analysis_core::{interleaved_to_mono, ChannelMix, OwnedAudioWaveformBatch};
use video_analysis_core::{OwnedAudioFrame, Result};
/// Re-exports the video analysis FFmpeg API.
pub use video_analysis_ffmpeg::{
    probe_audio as probe_audio_file, probe_audio_input, AudioMetadata, FfmpegAudioSource,
    FfmpegAudioSourceOptions,
};
/// Re-exports the video analysis ingest audio frame source API.
pub use video_analysis_ingest::{AudioFrameSource, AudioStreamInfo, SourceMode};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Variants describing audio input.
pub enum AudioInput {
    /// The file variant.
    File(PathBuf),
    /// The input variant.
    Input(String),
    /// The live variant.
    Live(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for audio input options.
pub struct AudioInputOptions {
    /// The samples per chunk value.
    pub samples_per_chunk: usize,
    /// The realtime value.
    pub realtime: bool,
    /// The extra input args value.
    pub extra_input_args: Vec<String>,
}

impl AudioInputOptions {
    /// Returns recorded.
    pub fn recorded() -> Self {
        Self {
            samples_per_chunk: 16_384,
            realtime: false,
            extra_input_args: Vec::new(),
        }
    }

    /// Returns live.
    pub fn live() -> Self {
        Self {
            samples_per_chunk: 1024,
            realtime: true,
            extra_input_args: Vec::new(),
        }
    }

    /// Returns samples per chunk.
    pub fn samples_per_chunk(mut self, samples: usize) -> Self {
        self.samples_per_chunk = samples.max(1);
        self
    }

    /// Returns realtime.
    pub fn realtime(mut self, realtime: bool) -> Self {
        self.realtime = realtime;
        self
    }

    /// Returns extra input arg.
    pub fn extra_input_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_input_args.push(arg.into());
        self
    }

    /// Consumes this value into a FFmpeg options.
    pub fn into_ffmpeg_options(self, mode: SourceMode) -> FfmpegAudioSourceOptions {
        let mut options = if mode == SourceMode::Live || self.realtime {
            FfmpegAudioSourceOptions::live()
        } else {
            FfmpegAudioSourceOptions::recorded()
        }
        .samples_per_chunk(self.samples_per_chunk);
        for arg in self.extra_input_args {
            options = options.extra_input_arg(arg);
        }
        options
    }
}

impl Default for AudioInputOptions {
    fn default() -> Self {
        Self::recorded()
    }
}

/// Returns open audio input.
pub fn open_audio_input(
    input: AudioInput,
    options: AudioInputOptions,
) -> Result<FfmpegAudioSource> {
    match input {
        AudioInput::File(path) => {
            let options = options.into_ffmpeg_options(SourceMode::Recorded);
            FfmpegAudioSource::open_path_with_options(path, options)
        }
        AudioInput::Input(input) => {
            let options = options.into_ffmpeg_options(SourceMode::Recorded);
            FfmpegAudioSource::open_input_with_options(input, options)
        }
        AudioInput::Live(input) => {
            let options = options.into_ffmpeg_options(SourceMode::Live);
            FfmpegAudioSource::open_input_with_options(input, options)
        }
    }
}

/// Returns probe audio input metadata.
pub fn probe_audio_input_metadata(input: &AudioInput) -> Result<AudioMetadata> {
    match input {
        AudioInput::File(path) => probe_audio_file(path).map_err(|err| {
            video_analysis_core::DetectError::Source(format!(
                "failed to probe audio file `{}`: {err}",
                path.display()
            ))
        }),
        AudioInput::Input(input) | AudioInput::Live(input) => {
            probe_audio_input(input).map_err(|err| {
                video_analysis_core::DetectError::Source(format!(
                    "failed to probe audio input `{input}`: {err}"
                ))
            })
        }
    }
}

/// Returns open audio input with metadata.
pub fn open_audio_input_with_metadata(
    input: AudioInput,
    options: AudioInputOptions,
) -> Result<(AudioMetadata, FfmpegAudioSource)> {
    let metadata = probe_audio_input_metadata(&input)?;
    let source = open_audio_input(input, options)?;
    Ok((metadata, source))
}

/// Returns decode audio to f32.
pub fn decode_audio_to_f32(
    input: AudioInput,
    options: AudioInputOptions,
) -> Result<(AudioMetadata, Vec<OwnedAudioFrame>)> {
    let (metadata, mut source) = open_audio_input_with_metadata(input, options)?;
    let mut frames = Vec::new();
    while let Some(frame) = source.next_audio_frame()? {
        frames.push(frame);
    }
    Ok((metadata, frames))
}

/// Returns decode audio to waveform batch.
pub fn decode_audio_to_waveform_batch(
    input: AudioInput,
    options: AudioInputOptions,
) -> Result<(AudioMetadata, OwnedAudioWaveformBatch)> {
    let (metadata, frames) = decode_audio_to_f32(input, options)?;
    let batch = OwnedAudioWaveformBatch::from_audio_frames(&frames)?;
    Ok((metadata, batch))
}

/// Returns decode audio to mono f32.
pub fn decode_audio_to_mono_f32(
    input: AudioInput,
    options: AudioInputOptions,
    mix: ChannelMix,
) -> Result<(AudioMetadata, Vec<f32>)> {
    let (metadata, frames) = decode_audio_to_f32(input, options)?;
    let mut mono = Vec::new();
    for frame in frames {
        let samples = interleaved_to_mono(&frame.data, frame.channels, mix)?;
        mono.extend(samples);
    }
    Ok((metadata, mono))
}

/// Writes waveform batch as wav.
pub fn write_waveform_batch_as_wav(
    path: impl AsRef<Path>,
    batch: &OwnedAudioWaveformBatch,
) -> Result<()> {
    let view = batch.as_view()?;
    if view.batch_size() != 1 {
        return Err(video_analysis_core::DetectError::InvalidArgument(
            "waveform WAV export requires a batch size of 1".to_string(),
        ));
    }
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let spec = hound::WavSpec {
        channels: view.channel_count() as u16,
        sample_rate: view.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(|err| {
        video_analysis_core::DetectError::Source(format!(
            "failed to create WAV `{}`: {err}",
            path.display()
        ))
    })?;
    for time_index in 0..view.time_steps() {
        for channel_index in 0..view.channel_count() {
            let sample = view.waveform(0, channel_index)?[time_index];
            writer.write_sample(sample).map_err(|err| {
                video_analysis_core::DetectError::Source(format!(
                    "failed to write WAV sample `{}`: {err}",
                    path.display()
                ))
            })?;
        }
    }
    writer.finalize().map_err(|err| {
        video_analysis_core::DetectError::Source(format!(
            "failed to finalize WAV `{}`: {err}",
            path.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use video_analysis_core::{AudioBuffer, Timebase, Timestamp};

    #[test]
    fn recorded_options_map_to_ffmpeg_recorded_mode() {
        let options = AudioInputOptions::recorded()
            .samples_per_chunk(4096)
            .extra_input_arg("-safe")
            .extra_input_arg("0")
            .into_ffmpeg_options(SourceMode::Recorded);

        assert_eq!(options.mode, SourceMode::Recorded);
        assert!(!options.realtime);
        assert_eq!(options.samples_per_chunk, 4096);
        assert_eq!(options.extra_input_args, vec!["-safe", "0"]);
    }

    #[test]
    fn live_options_map_to_ffmpeg_live_mode() {
        let options = AudioInputOptions::live().into_ffmpeg_options(SourceMode::Live);

        assert_eq!(options.mode, SourceMode::Live);
        assert!(options.realtime);
        assert_eq!(options.samples_per_chunk, 1024);
    }

    #[test]
    fn chunk_size_defaults_differ_for_live_and_recorded() {
        assert_eq!(AudioInputOptions::recorded().samples_per_chunk, 16_384);
        assert_eq!(AudioInputOptions::live().samples_per_chunk, 1024);
    }

    #[test]
    fn realtime_recorded_input_uses_live_ffmpeg_options() {
        let options = AudioInputOptions::recorded()
            .realtime(true)
            .into_ffmpeg_options(SourceMode::Recorded);

        assert_eq!(options.mode, SourceMode::Live);
        assert!(options.realtime);
    }

    #[test]
    fn samples_per_chunk_is_clamped_to_at_least_one() {
        assert_eq!(
            AudioInputOptions::recorded()
                .samples_per_chunk(0)
                .samples_per_chunk,
            1
        );
    }

    #[test]
    fn input_variants_choose_recorded_or_live_modes() {
        let cases = [
            (AudioInput::File("audio.wav".into()), SourceMode::Recorded),
            (
                AudioInput::Input("pipe:0".to_string()),
                SourceMode::Recorded,
            ),
            (
                AudioInput::Live("rtsp://example.test/live".to_string()),
                SourceMode::Live,
            ),
        ];
        for (input, expected_mode) in cases {
            let requested_mode = match input {
                AudioInput::File(_) | AudioInput::Input(_) => SourceMode::Recorded,
                AudioInput::Live(_) => SourceMode::Live,
            };
            let options = AudioInputOptions::default().into_ffmpeg_options(requested_mode);
            assert_eq!(options.mode, expected_mode);
        }
    }

    #[test]
    fn metadata_probe_errors_include_input_context() {
        let error = probe_audio_input_metadata(&AudioInput::File("missing.wav".into()))
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing.wav"));
    }

    #[test]
    fn writes_single_item_waveform_batches_to_wav() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("waveform.wav");
        let frame = OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 16_000)),
            16_000,
            2,
            AudioBuffer::F32(vec![0.0, 0.5, -0.25, 0.25]),
        )
        .unwrap();
        let batch = OwnedAudioWaveformBatch::from_audio_frames(&[frame]).unwrap();

        write_waveform_batch_as_wav(&path, &batch).unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let samples = reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(samples, vec![0.0, 0.5, -0.25, 0.25]);
    }

    #[test]
    fn rejects_multi_item_waveform_batches_for_wav_export() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("waveform.wav");
        let first = OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 16_000)),
            16_000,
            1,
            AudioBuffer::F32(vec![0.0, 0.25]),
        )
        .unwrap();
        let second = OwnedAudioFrame::new(
            Timestamp::new(2, Timebase::new(1, 16_000)),
            16_000,
            1,
            AudioBuffer::F32(vec![0.5, 0.75]),
        )
        .unwrap();
        let batch = OwnedAudioWaveformBatch::from_audio_frames(&[first, second]).unwrap();
        assert!(write_waveform_batch_as_wav(&path, &batch).is_err());
    }
}
