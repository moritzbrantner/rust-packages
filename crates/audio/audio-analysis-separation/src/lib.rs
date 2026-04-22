use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stem {
    Vocals,
    Drums,
    Bass,
    Other,
    Guitar,
    Piano,
    NoVocals,
    Custom(String),
}

impl Stem {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Vocals => "vocals",
            Self::Drums => "drums",
            Self::Bass => "bass",
            Self::Other => "other",
            Self::Guitar => "guitar",
            Self::Piano => "piano",
            Self::NoVocals => "no_vocals",
            Self::Custom(value) => value.as_str(),
        }
    }

    pub fn file_name(&self) -> String {
        format!("{}.wav", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HtdemucsOptions {
    pub command: PathBuf,
    pub command_args: Vec<OsString>,
    pub model: String,
    pub output_dir: PathBuf,
    pub two_stems: Option<Stem>,
    pub device: Option<String>,
    pub shifts: Option<u32>,
    pub overlap: Option<f32>,
    pub extra_args: Vec<OsString>,
}

impl HtdemucsOptions {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            ..Self::default()
        }
    }

    pub fn command(mut self, command: impl Into<PathBuf>) -> Self {
        self.command = command.into();
        self
    }

    pub fn command_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.command_args.push(arg.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn two_stems(mut self, stem: Stem) -> Self {
        self.two_stems = Some(stem);
        self
    }

    pub fn device(mut self, device: impl Into<String>) -> Self {
        self.device = Some(device.into());
        self
    }

    pub fn shifts(mut self, shifts: u32) -> Self {
        self.shifts = Some(shifts);
        self
    }

    pub fn overlap(mut self, overlap: f32) -> Self {
        self.overlap = Some(overlap);
        self
    }

    pub fn extra_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.command.as_os_str().is_empty() {
            return Err(DetectError::InvalidArgument(
                "demucs command must not be empty".to_string(),
            ));
        }
        if self.model.trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "demucs model must not be empty".to_string(),
            ));
        }
        if let Some(overlap) = self.overlap {
            if !overlap.is_finite() || !(0.0..1.0).contains(&overlap) {
                return Err(DetectError::InvalidArgument(
                    "demucs overlap must be finite and in the range [0, 1)".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for HtdemucsOptions {
    fn default() -> Self {
        Self {
            command: PathBuf::from("demucs"),
            command_args: Vec::new(),
            model: "htdemucs".to_string(),
            output_dir: PathBuf::from("separated"),
            two_stems: None,
            device: None,
            shifts: None,
            overlap: None,
            extra_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparatedStem {
    pub stem: Stem,
    pub path: PathBuf,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparationResult {
    pub input: PathBuf,
    pub model: String,
    pub output_dir: PathBuf,
    pub stems: Vec<SeparatedStem>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HtdemucsSeparator {
    pub options: HtdemucsOptions,
}

impl HtdemucsSeparator {
    pub fn new(options: HtdemucsOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self { options })
    }

    pub fn build_args(&self, input: impl AsRef<Path>) -> Result<Vec<OsString>> {
        self.options.validate()?;
        let mut args = Vec::new();
        args.extend(self.options.command_args.iter().cloned());
        args.push(OsString::from("-n"));
        args.push(OsString::from(&self.options.model));
        args.push(OsString::from("-o"));
        args.push(self.options.output_dir.as_os_str().to_os_string());
        if let Some(stem) = &self.options.two_stems {
            args.push(OsString::from("--two-stems"));
            args.push(OsString::from(stem.as_str()));
        }
        if let Some(device) = &self.options.device {
            args.push(OsString::from("--device"));
            args.push(OsString::from(device));
        }
        if let Some(shifts) = self.options.shifts {
            args.push(OsString::from("--shifts"));
            args.push(OsString::from(shifts.to_string()));
        }
        if let Some(overlap) = self.options.overlap {
            args.push(OsString::from("--overlap"));
            args.push(OsString::from(overlap.to_string()));
        }
        args.extend(self.options.extra_args.iter().cloned());
        args.push(input.as_ref().as_os_str().to_os_string());
        Ok(args)
    }

    pub fn separate(&self, input: impl AsRef<Path>) -> Result<SeparationResult> {
        let input = input.as_ref();
        self.options.validate()?;
        let status = Command::new(&self.options.command)
            .args(self.build_args(input)?)
            .stdin(Stdio::null())
            .status()
            .map_err(|err| {
                DetectError::Source(format!(
                    "failed to start demucs command `{}`: {err}",
                    self.options.command.display()
                ))
            })?;
        if !status.success() {
            return Err(DetectError::Source(format!(
                "demucs command exited with status {status}"
            )));
        }
        Ok(self.expected_result(input))
    }

    pub fn expected_result(&self, input: impl AsRef<Path>) -> SeparationResult {
        let input = input.as_ref();
        let source_name = input
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("audio");
        let output_dir = self
            .options
            .output_dir
            .join(&self.options.model)
            .join(source_name);
        let stems = self
            .expected_stems()
            .into_iter()
            .map(|stem| {
                let path = output_dir.join(stem.file_name());
                let exists = path.exists();
                SeparatedStem { stem, path, exists }
            })
            .collect();
        SeparationResult {
            input: input.to_path_buf(),
            model: self.options.model.clone(),
            output_dir,
            stems,
        }
    }

    pub fn expected_stems(&self) -> Vec<Stem> {
        match &self.options.two_stems {
            Some(Stem::Vocals) => vec![Stem::Vocals, Stem::NoVocals],
            Some(stem) => vec![stem.clone(), Stem::Custom(format!("no_{}", stem.as_str()))],
            None => vec![Stem::Vocals, Stem::Drums, Stem::Bass, Stem::Other],
        }
    }
}

pub fn is_demucs_available() -> bool {
    Command::new("demucs")
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_as_strings(args: Vec<OsString>) -> Vec<String> {
        args.into_iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn builds_htdemucs_command_arguments() {
        let separator = HtdemucsSeparator::new(
            HtdemucsOptions::new("out")
                .two_stems(Stem::Vocals)
                .device("cpu")
                .shifts(2)
                .overlap(0.25),
        )
        .unwrap();
        let args = args_as_strings(separator.build_args("song.wav").unwrap());
        assert_eq!(
            args,
            vec![
                "-n",
                "htdemucs",
                "-o",
                "out",
                "--two-stems",
                "vocals",
                "--device",
                "cpu",
                "--shifts",
                "2",
                "--overlap",
                "0.25",
                "song.wav"
            ]
        );
    }

    #[test]
    fn predicts_standard_htdemucs_stem_paths() {
        let separator = HtdemucsSeparator::new(HtdemucsOptions::new("out")).unwrap();
        let result = separator.expected_result("/tmp/song.mp3");
        assert_eq!(result.output_dir, PathBuf::from("out/htdemucs/song"));
        assert_eq!(
            result
                .stems
                .iter()
                .map(|stem| stem.stem.as_str())
                .collect::<Vec<_>>(),
            vec!["vocals", "drums", "bass", "other"]
        );
        assert_eq!(
            result.stems[0].path,
            PathBuf::from("out/htdemucs/song/vocals.wav")
        );
    }
}
