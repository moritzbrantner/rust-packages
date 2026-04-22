use std::path::PathBuf;

use video_analysis_core::Result;
pub use video_analysis_ffmpeg::{AudioMetadata, FfmpegAudioSource, FfmpegAudioSourceOptions};
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_options_map_to_ffmpeg_recorded_mode() {
        let options = AudioInputOptions::recorded()
            .samples_per_chunk(4096)
            .extra_input_arg("-safe")
            .into_ffmpeg_options(SourceMode::Recorded);

        assert_eq!(options.mode, SourceMode::Recorded);
        assert!(!options.realtime);
        assert_eq!(options.samples_per_chunk, 4096);
        assert_eq!(options.extra_input_args, vec!["-safe"]);
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
}
