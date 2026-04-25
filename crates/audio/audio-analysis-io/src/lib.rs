#![doc = include_str!("../README.md")]

use std::path::PathBuf;

use audio_analysis_core::{interleaved_to_mono, ChannelMix};
use video_analysis_core::{OwnedAudioFrame, Result};
pub use video_analysis_ffmpeg::{
    probe_audio as probe_audio_file, probe_audio_input, AudioMetadata, FfmpegAudioSource,
    FfmpegAudioSourceOptions,
};
pub use video_analysis_ingest::{AudioFrameSource, AudioStreamInfo, SourceMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioInput {
    File(PathBuf),
    Input(String),
    Live(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioInputOptions {
    pub samples_per_chunk: usize,
    pub realtime: bool,
    pub extra_input_args: Vec<String>,
}

impl AudioInputOptions {
    pub fn recorded() -> Self {
        Self {
            samples_per_chunk: 16_384,
            realtime: false,
            extra_input_args: Vec::new(),
        }
    }

    pub fn live() -> Self {
        Self {
            samples_per_chunk: 1024,
            realtime: true,
            extra_input_args: Vec::new(),
        }
    }

    pub fn samples_per_chunk(mut self, samples: usize) -> Self {
        self.samples_per_chunk = samples.max(1);
        self
    }

    pub fn realtime(mut self, realtime: bool) -> Self {
        self.realtime = realtime;
        self
    }

    pub fn extra_input_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_input_args.push(arg.into());
        self
    }

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

pub fn open_audio_input_with_metadata(
    input: AudioInput,
    options: AudioInputOptions,
) -> Result<(AudioMetadata, FfmpegAudioSource)> {
    let metadata = probe_audio_input_metadata(&input)?;
    let source = open_audio_input(input, options)?;
    Ok((metadata, source))
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
