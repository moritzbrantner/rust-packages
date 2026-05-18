#![doc = include_str!("../README.md")]

use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use video_analysis_core::{DetectError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Variants describing stem.
pub enum Stem {
    /// The vocals variant.
    Vocals,
    /// The drums variant.
    Drums,
    /// The bass variant.
    Bass,
    /// The other variant.
    Other,
    /// The guitar variant.
    Guitar,
    /// The piano variant.
    Piano,
    /// The no vocals variant.
    NoVocals,
    /// The custom variant.
    Custom(String),
}

impl Stem {
    /// Borrows this value as a str.
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

    /// Returns residual for.
    pub fn residual_for(primary: &Stem) -> Stem {
        match primary {
            Self::Vocals => Self::NoVocals,
            stem => Self::Custom(format!("no_{}", stem.as_str())),
        }
    }

    /// Returns file name.
    pub fn file_name(&self, format: &SeparationOutputFormat) -> String {
        format!("{}.{}", self.as_str(), format.extension())
    }
}

impl Display for Stem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Stem {
    type Err = DetectError;

    fn from_str(input: &str) -> Result<Self> {
        let normalized = input.trim().to_ascii_lowercase().replace('-', "_");
        if normalized.is_empty() {
            return Err(DetectError::InvalidArgument(
                "stem must not be empty".to_string(),
            ));
        }
        Ok(match normalized.as_str() {
            "vocals" => Self::Vocals,
            "drums" => Self::Drums,
            "bass" => Self::Bass,
            "other" => Self::Other,
            "guitar" => Self::Guitar,
            "piano" => Self::Piano,
            "no_vocals" => Self::NoVocals,
            _ => Self::Custom(normalized),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
/// Variants describing demucs model.
pub enum DemucsModel {
    #[default]
    /// The htdemucs variant.
    Htdemucs,
    /// The htdemucs ft variant.
    HtdemucsFt,
    /// The htdemucs6s variant.
    Htdemucs6s,
    /// The md x variant.
    MdX,
    /// The md x extra variant.
    MdXExtra,
    /// The md xq variant.
    MdXQ,
    /// The custom variant.
    Custom(String),
}

impl DemucsModel {
    /// Borrows this value as a str.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Htdemucs => "htdemucs",
            Self::HtdemucsFt => "htdemucs_ft",
            Self::Htdemucs6s => "htdemucs_6s",
            Self::MdX => "mdx",
            Self::MdXExtra => "mdx_extra",
            Self::MdXQ => "mdx_q",
            Self::Custom(value) => value.as_str(),
        }
    }

    /// Returns default layout.
    pub fn default_layout(&self) -> StemLayout {
        match self {
            Self::Htdemucs6s => StemLayout::SixStem,
            Self::Htdemucs
            | Self::HtdemucsFt
            | Self::MdX
            | Self::MdXExtra
            | Self::MdXQ
            | Self::Custom(_) => StemLayout::FourStem,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.as_str().trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "demucs model must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

impl Display for DemucsModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DemucsModel {
    type Err = DetectError;

    fn from_str(input: &str) -> Result<Self> {
        let normalized = input.trim().to_ascii_lowercase().replace('-', "_");
        if normalized.is_empty() {
            return Err(DetectError::InvalidArgument(
                "demucs model must not be empty".to_string(),
            ));
        }
        Ok(match normalized.as_str() {
            "htdemucs" => Self::Htdemucs,
            "htdemucs_ft" => Self::HtdemucsFt,
            "htdemucs_6s" => Self::Htdemucs6s,
            "mdx" => Self::MdX,
            "mdx_extra" => Self::MdXExtra,
            "mdx_q" => Self::MdXQ,
            _ => Self::Custom(normalized),
        })
    }
}

impl From<String> for DemucsModel {
    fn from(value: String) -> Self {
        Self::from_str(&value).unwrap_or(Self::Custom(value))
    }
}

impl From<&str> for DemucsModel {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Variants describing stem layout.
pub enum StemLayout {
    /// The four stem variant.
    FourStem,
    /// The six stem variant.
    SixStem,
    /// The two stem variant.
    TwoStem {
        /// The primary value for this variant.
        primary: Stem,
        /// The residual value for this variant.
        residual: Stem,
    },
    /// The custom variant.
    Custom(Vec<Stem>),
}

impl StemLayout {
    /// Returns stems.
    pub fn stems(&self) -> Vec<Stem> {
        match self {
            Self::FourStem => vec![Stem::Vocals, Stem::Drums, Stem::Bass, Stem::Other],
            Self::SixStem => vec![
                Stem::Vocals,
                Stem::Drums,
                Stem::Bass,
                Stem::Other,
                Stem::Guitar,
                Stem::Piano,
            ],
            Self::TwoStem { primary, residual } => vec![primary.clone(), residual.clone()],
            Self::Custom(stems) => stems.clone(),
        }
    }

    fn validate(&self) -> Result<()> {
        if let Self::Custom(stems) = self {
            if stems.is_empty() {
                return Err(DetectError::InvalidArgument(
                    "custom stem layout must contain at least one stem".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// Variants describing separation output format.
pub enum SeparationOutputFormat {
    #[default]
    /// The wav variant.
    Wav,
    /// The mp3 variant.
    Mp3,
    /// The flac variant.
    Flac,
    /// The custom variant.
    Custom(String),
}

impl SeparationOutputFormat {
    /// Returns extension.
    pub fn extension(&self) -> &str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Custom(ext) => ext,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.extension().trim().is_empty() {
            return Err(DetectError::InvalidArgument(
                "output format extension must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing separation run mode.
pub enum SeparationRunMode {
    /// The execute variant.
    Execute,
    /// The dry run variant.
    DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for separation command.
pub struct SeparationCommand {
    /// The program value.
    pub program: PathBuf,
    /// The args value.
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for separated stem.
pub struct SeparatedStem {
    /// The stem value.
    pub stem: Stem,
    /// Filesystem path for this value.
    pub path: PathBuf,
    /// The exists value.
    pub exists: bool,
    /// The bytes value.
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for separation result.
pub struct SeparationResult {
    /// The input value.
    pub input: PathBuf,
    /// The model value.
    pub model: DemucsModel,
    /// The layout value.
    pub layout: StemLayout,
    /// The output dir value.
    pub output_dir: PathBuf,
    /// The stems value.
    pub stems: Vec<SeparatedStem>,
    /// The missing stems value.
    pub missing_stems: Vec<Stem>,
    /// The all outputs present value.
    pub all_outputs_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Data type for separation execution.
pub struct SeparationExecution {
    /// The command value.
    pub command: SeparationCommand,
    /// The result value.
    pub result: SeparationResult,
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for htdemucs options.
pub struct HtdemucsOptions {
    /// The command value.
    pub command: PathBuf,
    /// The command args value.
    pub command_args: Vec<OsString>,
    /// The model value.
    pub model: DemucsModel,
    /// The output dir value.
    pub output_dir: PathBuf,
    /// The layout value.
    pub layout: Option<StemLayout>,
    /// The output format value.
    pub output_format: SeparationOutputFormat,
    /// The two stems value.
    pub two_stems: Option<Stem>,
    /// The device value.
    pub device: Option<String>,
    /// The shifts value.
    pub shifts: Option<u32>,
    /// The overlap value.
    pub overlap: Option<f32>,
    /// The jobs value.
    pub jobs: Option<u32>,
    /// The segment value.
    pub segment: Option<u32>,
    /// Sample rate in hertz.
    pub sample_rate: Option<u32>,
    /// The filename value.
    pub filename: Option<String>,
    /// The extra args value.
    pub extra_args: Vec<OsString>,
}

impl HtdemucsOptions {
    /// Creates a new value.
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            ..Self::default()
        }
    }

    /// Returns command.
    pub fn command(mut self, command: impl Into<PathBuf>) -> Self {
        self.command = command.into();
        self
    }

    /// Returns command arg.
    pub fn command_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.command_args.push(arg.into());
        self
    }

    /// Returns model.
    pub fn model(mut self, model: impl Into<DemucsModel>) -> Self {
        self.model = model.into();
        self
    }

    /// Returns layout.
    pub fn layout(mut self, layout: StemLayout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Returns output format.
    pub fn output_format(mut self, output_format: SeparationOutputFormat) -> Self {
        self.output_format = output_format;
        self
    }

    /// Returns two stems.
    pub fn two_stems(mut self, stem: Stem) -> Self {
        self.two_stems = Some(stem);
        self
    }

    /// Returns device.
    pub fn device(mut self, device: impl Into<String>) -> Self {
        self.device = Some(device.into());
        self
    }

    /// Returns shifts.
    pub fn shifts(mut self, shifts: u32) -> Self {
        self.shifts = Some(shifts);
        self
    }

    /// Returns overlap.
    pub fn overlap(mut self, overlap: f32) -> Self {
        self.overlap = Some(overlap);
        self
    }

    /// Returns jobs.
    pub fn jobs(mut self, jobs: u32) -> Self {
        self.jobs = Some(jobs);
        self
    }

    /// Returns segment.
    pub fn segment(mut self, segment: u32) -> Self {
        self.segment = Some(segment);
        self
    }

    /// Returns sample rate.
    pub fn sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = Some(sample_rate);
        self
    }

    /// Returns filename.
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Returns extra arg.
    pub fn extra_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.extra_args.push(arg.into());
        self
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.command.as_os_str().is_empty() {
            return Err(DetectError::InvalidArgument(
                "demucs command must not be empty".to_string(),
            ));
        }
        self.model.validate()?;
        self.output_format.validate()?;
        if let Some(layout) = &self.layout {
            layout.validate()?;
        }
        if self.two_stems.is_some() && self.layout.is_some() {
            return Err(DetectError::InvalidArgument(
                "custom layout cannot be combined with two_stems".to_string(),
            ));
        }
        if let Some(overlap) = self.overlap {
            if !overlap.is_finite() || !(0.0..1.0).contains(&overlap) {
                return Err(DetectError::InvalidArgument(
                    "demucs overlap must be finite and in the range [0, 1)".to_string(),
                ));
            }
        }
        if self.jobs == Some(0) {
            return Err(DetectError::InvalidArgument(
                "demucs jobs must be greater than zero".to_string(),
            ));
        }
        if self.segment == Some(0) {
            return Err(DetectError::InvalidArgument(
                "demucs segment must be greater than zero".to_string(),
            ));
        }
        if self.sample_rate == Some(0) {
            return Err(DetectError::InvalidArgument(
                "demucs sample_rate must be greater than zero".to_string(),
            ));
        }
        if self
            .filename
            .as_ref()
            .is_some_and(|filename| filename.trim().is_empty())
        {
            return Err(DetectError::InvalidArgument(
                "demucs filename template must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for HtdemucsOptions {
    fn default() -> Self {
        Self {
            command: PathBuf::from("demucs"),
            command_args: Vec::new(),
            model: DemucsModel::default(),
            output_dir: PathBuf::from("separated"),
            layout: None,
            output_format: SeparationOutputFormat::Wav,
            two_stems: None,
            device: None,
            shifts: None,
            overlap: None,
            jobs: None,
            segment: None,
            sample_rate: None,
            filename: None,
            extra_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Data type for htdemucs separator.
pub struct HtdemucsSeparator {
    /// The options value.
    pub options: HtdemucsOptions,
}

impl HtdemucsSeparator {
    /// Creates a new value.
    pub fn new(options: HtdemucsOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self { options })
    }

    /// Validates input path.
    pub fn validate_input_path(&self, input: &Path) -> Result<()> {
        if input.as_os_str().is_empty() {
            return Err(DetectError::InvalidArgument(
                "audio input path must not be empty".to_string(),
            ));
        }
        if input.file_stem().is_none() {
            return Err(DetectError::InvalidArgument(
                "audio input path must include a file name".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns expected layout.
    pub fn expected_layout(&self) -> StemLayout {
        if let Some(primary) = &self.options.two_stems {
            return StemLayout::TwoStem {
                primary: primary.clone(),
                residual: Stem::residual_for(primary),
            };
        }
        self.options
            .layout
            .clone()
            .unwrap_or_else(|| self.options.model.default_layout())
    }

    /// Returns expected stems.
    pub fn expected_stems(&self) -> Vec<Stem> {
        self.expected_layout().stems()
    }

    /// Builds command.
    pub fn build_command(&self, input: impl AsRef<Path>) -> Result<SeparationCommand> {
        let input = input.as_ref();
        self.options.validate()?;
        self.validate_input_path(input)?;

        let mut args = Vec::new();
        args.extend(self.options.command_args.iter().cloned());
        args.push(OsString::from("-n"));
        args.push(OsString::from(self.options.model.as_str()));
        args.push(OsString::from("-o"));
        args.push(self.options.output_dir.as_os_str().to_os_string());
        match &self.options.output_format {
            SeparationOutputFormat::Wav => {}
            SeparationOutputFormat::Mp3 => args.push(OsString::from("--mp3")),
            SeparationOutputFormat::Flac => args.push(OsString::from("--flac")),
            SeparationOutputFormat::Custom(_) => {}
        }
        if let Some(filename) = &self.options.filename {
            args.push(OsString::from("--filename"));
            args.push(OsString::from(filename));
        }
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
        if let Some(jobs) = self.options.jobs {
            args.push(OsString::from("-j"));
            args.push(OsString::from(jobs.to_string()));
        }
        if let Some(segment) = self.options.segment {
            args.push(OsString::from("--segment"));
            args.push(OsString::from(segment.to_string()));
        }
        if let Some(sample_rate) = self.options.sample_rate {
            args.push(OsString::from("--samplerate"));
            args.push(OsString::from(sample_rate.to_string()));
        }
        args.extend(self.options.extra_args.iter().cloned());
        args.push(input.as_os_str().to_os_string());

        Ok(SeparationCommand {
            program: self.options.command.clone(),
            args,
        })
    }

    /// Builds args.
    pub fn build_args(&self, input: impl AsRef<Path>) -> Result<Vec<OsString>> {
        Ok(self.build_command(input)?.args)
    }

    /// Returns dry run.
    pub fn dry_run(&self, input: impl AsRef<Path>) -> Result<SeparationExecution> {
        let input = input.as_ref();
        Ok(SeparationExecution {
            command: self.build_command(input)?,
            result: self.discover_result(input)?,
        })
    }

    /// Returns separate.
    pub fn separate(&self, input: impl AsRef<Path>) -> Result<SeparationResult> {
        let input = input.as_ref();
        self.options.validate()?;
        self.validate_input_path(input)?;
        if !input.is_file() {
            return Err(DetectError::Source(format!(
                "demucs input `{}` does not exist or is not a file",
                input.display()
            )));
        }

        let command = self.build_command(input)?;
        let status = Command::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::null())
            .status()
            .map_err(|err| {
                DetectError::Source(format!(
                    "failed to start demucs command `{}`: {err}",
                    command.program.display()
                ))
            })?;
        if !status.success() {
            return Err(DetectError::Source(format!(
                "demucs command exited with status {status}"
            )));
        }

        let result = self.discover_result(input)?;
        if !result.all_outputs_present {
            return Err(DetectError::Source(format!(
                "demucs completed but missing expected outputs: {}",
                result
                    .missing_stems
                    .iter()
                    .map(Stem::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        Ok(result)
    }

    /// Returns discover result.
    pub fn discover_result(&self, input: impl AsRef<Path>) -> Result<SeparationResult> {
        let input = input.as_ref();
        self.options.validate()?;
        self.validate_input_path(input)?;

        let source_name = input
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("audio");
        let layout = self.expected_layout();
        let model_dir = self.options.output_dir.join(self.options.model.as_str());
        let output_dir = if self.options.filename.is_some() {
            model_dir.clone()
        } else {
            model_dir.join(source_name)
        };
        let stems = layout
            .stems()
            .into_iter()
            .map(|stem| {
                let path = self.output_path_for_stem(&model_dir, source_name, &stem);
                let bytes = file_size_if_nonempty(&path);
                let exists = bytes.is_some();
                SeparatedStem {
                    stem,
                    path,
                    exists,
                    bytes,
                }
            })
            .collect::<Vec<_>>();
        let missing_stems = stems
            .iter()
            .filter(|stem| !stem.exists)
            .map(|stem| stem.stem.clone())
            .collect::<Vec<_>>();

        Ok(SeparationResult {
            input: input.to_path_buf(),
            model: self.options.model.clone(),
            layout,
            output_dir,
            all_outputs_present: missing_stems.is_empty(),
            missing_stems,
            stems,
        })
    }

    /// Returns expected result.
    pub fn expected_result(&self, input: impl AsRef<Path>) -> SeparationResult {
        self.discover_result(input)
            .expect("separator expected_result uses validated static path computation")
    }

    fn output_path_for_stem(&self, model_dir: &Path, source_name: &str, stem: &Stem) -> PathBuf {
        if let Some(template) = &self.options.filename {
            let rendered = render_filename_template(
                template,
                source_name,
                stem.as_str(),
                self.options.output_format.extension(),
                self.options.model.as_str(),
            );
            return model_dir.join(rendered);
        }
        model_dir
            .join(source_name)
            .join(stem.file_name(&self.options.output_format))
    }
}

/// Returns whether is demucs available.
pub fn is_demucs_available() -> bool {
    let command = std::env::var_os("DEMUCS_COMMAND").unwrap_or_else(|| OsString::from("demucs"));
    Command::new(command)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn render_filename_template(
    template: &str,
    track: &str,
    stem: &str,
    ext: &str,
    model: &str,
) -> PathBuf {
    PathBuf::from(
        template
            .replace("{track}", track)
            .replace("{stem}", stem)
            .replace("{ext}", ext)
            .replace("{model}", model),
    )
}

fn file_size_if_nonempty(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file() && metadata.len() > 0)
        .map(|metadata| metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_parsing_normalizes_known_and_custom_values() {
        assert_eq!("vocals".parse::<Stem>().unwrap(), Stem::Vocals);
        assert_eq!("no-vocals".parse::<Stem>().unwrap(), Stem::NoVocals);
        assert_eq!(
            " Lead-Guitar ".parse::<Stem>().unwrap(),
            Stem::Custom("lead_guitar".to_string())
        );
        assert!(" ".parse::<Stem>().is_err());
    }

    #[test]
    fn custom_model_and_format_validation_rejects_empty_values() {
        assert!(DemucsModel::Custom(" ".to_string()).validate().is_err());
        assert!(SeparationOutputFormat::Custom(" ".to_string())
            .validate()
            .is_err());
    }

    #[test]
    fn filename_template_renders_track_stem_extension_and_model() {
        assert_eq!(
            render_filename_template(
                "{model}/{track}/{stem}.{ext}",
                "song",
                "vocals",
                "flac",
                "htdemucs",
            ),
            PathBuf::from("htdemucs/song/vocals.flac")
        );
    }

    #[test]
    fn separator_rejects_inputs_without_file_names() {
        let separator = HtdemucsSeparator::new(HtdemucsOptions::default()).unwrap();

        assert!(separator.validate_input_path(Path::new("")).is_err());
        assert!(separator.validate_input_path(Path::new("/")).is_err());
        assert!(separator.validate_input_path(Path::new("song.wav")).is_ok());
    }
}
