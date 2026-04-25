mod ffi;

use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WhisperCppModel {
    #[serde(rename = "tiny.en")]
    TinyEn,
    #[serde(rename = "tiny")]
    Tiny,
    #[serde(rename = "base.en")]
    BaseEn,
    #[serde(rename = "base")]
    Base,
    #[serde(rename = "small.en")]
    SmallEn,
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "medium.en")]
    MediumEn,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "large-v1")]
    LargeV1,
    #[serde(rename = "large-v2")]
    LargeV2,
    #[serde(rename = "large-v3")]
    LargeV3,
    #[serde(rename = "large-v3-turbo")]
    LargeV3Turbo,
}

impl WhisperCppModel {
    pub const ALL: [Self; 12] = [
        Self::TinyEn,
        Self::Tiny,
        Self::BaseEn,
        Self::Base,
        Self::SmallEn,
        Self::Small,
        Self::MediumEn,
        Self::Medium,
        Self::LargeV1,
        Self::LargeV2,
        Self::LargeV3,
        Self::LargeV3Turbo,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::TinyEn => "tiny.en",
            Self::Tiny => "tiny",
            Self::BaseEn => "base.en",
            Self::Base => "base",
            Self::SmallEn => "small.en",
            Self::Small => "small",
            Self::MediumEn => "medium.en",
            Self::Medium => "medium",
            Self::LargeV1 => "large-v1",
            Self::LargeV2 => "large-v2",
            Self::LargeV3 => "large-v3",
            Self::LargeV3Turbo => "large-v3-turbo",
        }
    }

    pub fn file_name(self) -> String {
        format!("ggml-{}.bin", self.id())
    }

    pub fn download_url(self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.file_name()
        )
    }

    pub fn checksum_sha1(self) -> &'static str {
        match self {
            Self::TinyEn => "c78c86eb1a8faa21b369bcd33207cc90d64ae9df",
            Self::Tiny => "bd577a113a864445d4c299885e0cb97d4ba92b5f",
            Self::BaseEn => "137c40403d78fd54d454da0f9bd998f78703390c",
            Self::Base => "465707469ff3a37a2b9b8d8f89f2f99de7299dac",
            Self::SmallEn => "db8a495a91d927739e50b3fc1cc4c6b8f6c2d022",
            Self::Small => "55356645c2b361a969dfd0ef2c5a50d530afd8d5",
            Self::MediumEn => "8c30f0e44ce9560643ebd10bbe50cd20eafd3723",
            Self::Medium => "fd9727b6e1217c2f614f9b698455c4ffd82463b4",
            Self::LargeV1 => "b1caaf735c4cc1429223d5a74f0f4d0b9b59a299",
            Self::LargeV2 => "0f4c8e34f21cf1a914c59d8b3ce882345ad349d6",
            Self::LargeV3 => "ad82bf6a9043ceed055076d0fd39f5f186ff8062",
            Self::LargeV3Turbo => "4af2b29d7ec73d781377bfd1758ca957a807e941",
        }
    }

    pub fn multilingual(self) -> bool {
        !matches!(
            self,
            Self::TinyEn | Self::BaseEn | Self::SmallEn | Self::MediumEn
        )
    }
}

impl Default for WhisperCppModel {
    fn default() -> Self {
        Self::BaseEn
    }
}

impl Display for WhisperCppModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhisperCppConfig {
    #[serde(default)]
    pub model: WhisperCppModel,
    pub language: Option<String>,
    #[serde(default)]
    pub translate: bool,
    pub threads: Option<usize>,
}

impl Default for WhisperCppConfig {
    fn default() -> Self {
        Self {
            model: WhisperCppModel::default(),
            language: None,
            translate: false,
            threads: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhisperCppSegment {
    pub index: u64,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    pub text: String,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhisperCppTranscription {
    pub text: Option<String>,
    pub language: Option<String>,
    pub segments: Vec<WhisperCppSegment>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WhisperCppPhase {
    Preparing,
    DownloadingModel,
    LoadingModel,
    Transcribing,
}

impl WhisperCppPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::DownloadingModel => "downloading_model",
            Self::LoadingModel => "loading_model",
            Self::Transcribing => "transcribing",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhisperCppProgressEvent {
    pub phase: WhisperCppPhase,
    pub message: String,
    pub progress: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhisperCppModelStatus {
    pub model: WhisperCppModel,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WhisperCppCatalog {
    pub default_model: WhisperCppModel,
    pub models: Vec<WhisperCppModelStatus>,
}

#[derive(Debug, thiserror::Error)]
pub enum WhisperCppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("wave input error: {0}")]
    Wav(#[from] hound::Error),
    #[error("network error: {0}")]
    Http(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unsupported language `{0}`")]
    UnsupportedLanguage(String),
    #[error("downloaded model `{model}` failed checksum verification")]
    InvalidChecksum { model: WhisperCppModel },
    #[error("failed to initialize whisper.cpp from `{0}`")]
    Initialization(String),
    #[error("whisper.cpp inference failed for `{0}`")]
    Inference(String),
    #[error("invalid utf-8 returned by whisper.cpp")]
    InvalidUtf8,
}

pub type Result<T> = std::result::Result<T, WhisperCppError>;

type ProgressCallback = dyn FnMut(WhisperCppProgressEvent);

pub struct ModelStore {
    root: PathBuf,
}

impl Default for ModelStore {
    fn default() -> Self {
        Self {
            root: cache_root().join("whisper-cpp"),
        }
    }
}

impl ModelStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn model_path(&self, model: WhisperCppModel) -> PathBuf {
        self.models_dir().join(model.file_name())
    }

    pub fn lock_path(&self, model: WhisperCppModel) -> PathBuf {
        self.models_dir()
            .join(format!("{}.lock", model.file_name()))
    }

    pub fn catalog(&self) -> WhisperCppCatalog {
        WhisperCppCatalog {
            default_model: WhisperCppModel::default(),
            models: WhisperCppModel::ALL
                .into_iter()
                .map(|model| WhisperCppModelStatus {
                    model,
                    cached: self.model_path(model).is_file(),
                })
                .collect(),
        }
    }

    pub fn ensure_model(
        &self,
        model: WhisperCppModel,
        progress: Option<&mut ProgressCallback>,
    ) -> Result<PathBuf> {
        fs::create_dir_all(self.models_dir())?;
        let model_path = self.model_path(model);
        if model_path.is_file() {
            return Ok(model_path);
        }

        let _lock = FileLock::acquire(self.lock_path(model))?;
        if model_path.is_file() {
            return Ok(model_path);
        }

        emit_progress(
            progress,
            WhisperCppPhase::DownloadingModel,
            format!("downloading whisper.cpp model `{model}`"),
            Some(0.0),
        );

        let temp_path = model_path.with_extension("bin.part");
        if temp_path.exists() {
            let _ = fs::remove_file(&temp_path);
        }

        let response = ureq::get(&model.download_url())
            .call()
            .map_err(|error| WhisperCppError::Http(error.to_string()))?;
        let total_bytes = response
            .header("Content-Length")
            .and_then(|value| value.parse::<u64>().ok());
        let mut reader = response.into_reader();
        let mut file = BufWriter::new(File::create(&temp_path)?);
        let mut hasher = Sha1::new();
        let mut downloaded = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];

        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| WhisperCppError::Http(error.to_string()))?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read])?;
            hasher.update(&buffer[..read]);
            downloaded += read as u64;
            let fraction = total_bytes.map(|total| (downloaded as f32 / total as f32).clamp(0.0, 1.0));
            emit_progress(
                progress,
                WhisperCppPhase::DownloadingModel,
                format!("downloading whisper.cpp model `{model}`"),
                fraction,
            );
        }
        file.flush()?;

        let checksum = format!("{:x}", hasher.finalize());
        if checksum != model.checksum_sha1() {
            let _ = fs::remove_file(&temp_path);
            return Err(WhisperCppError::InvalidChecksum { model });
        }

        fs::rename(temp_path, &model_path)?;
        Ok(model_path)
    }
}

pub struct WhisperCppTranscriber {
    config: WhisperCppConfig,
    store: ModelStore,
    progress: Option<Box<ProgressCallback>>,
}

impl WhisperCppTranscriber {
    pub fn new(config: WhisperCppConfig) -> Self {
        Self {
            config,
            store: ModelStore::default(),
            progress: None,
        }
    }

    pub fn with_model_store(mut self, store: ModelStore) -> Self {
        self.store = store;
        self
    }

    pub fn on_progress<F>(mut self, callback: F) -> Self
    where
        F: FnMut(WhisperCppProgressEvent) + 'static,
    {
        self.progress = Some(Box::new(callback));
        self
    }

    pub fn transcribe_file(&mut self, input: &Path) -> Result<WhisperCppTranscription> {
        self.transcribe_file_with_optional_progress(input, None)
    }

    pub fn transcribe_file_with_progress(
        &mut self,
        input: &Path,
        progress: &mut ProgressCallback,
    ) -> Result<WhisperCppTranscription> {
        self.transcribe_file_with_optional_progress(input, Some(progress))
    }

    fn transcribe_file_with_optional_progress(
        &mut self,
        input: &Path,
        external_progress: Option<&mut ProgressCallback>,
    ) -> Result<WhisperCppTranscription> {
        let mut external_progress = external_progress;
        emit_progress(
            progress_target(external_progress.as_deref_mut(), self.progress_deref_mut()),
            WhisperCppPhase::Preparing,
            format!("preparing native whisper.cpp transcription for {}", input.display()),
            None,
        );

        let model_path = self
            .store
            .ensure_model(
                self.config.model,
                progress_target(external_progress.as_deref_mut(), self.progress_deref_mut()),
            )?;

        emit_progress(
            progress_target(external_progress.as_deref_mut(), self.progress_deref_mut()),
            WhisperCppPhase::LoadingModel,
            format!("loading whisper.cpp model `{}`", self.config.model),
            None,
        );

        let audio = read_wav_mono_f32(input)?;
        emit_progress(
            progress_target(external_progress.as_deref_mut(), self.progress_deref_mut()),
            WhisperCppPhase::Transcribing,
            format!("transcribing audio with whisper.cpp model `{}`", self.config.model),
            None,
        );

        let context = WhisperContext::from_model(&model_path)?;
        let mut params =
            unsafe { ffi::whisper_full_default_params(ffi::whisper_sampling_strategy::WHISPER_SAMPLING_GREEDY) };
        params.n_threads = resolve_threads(self.config.threads);
        params.translate = self.config.translate;
        params.print_progress = false;
        params.print_realtime = false;
        params.print_special = false;
        params.print_timestamps = false;
        params.no_timestamps = false;

        let language = match self.config.language.as_deref().filter(|value| !value.is_empty()) {
            Some(value) if value.eq_ignore_ascii_case("auto") => None,
            Some(value) => Some(
                CString::new(value)
                    .map_err(|_| WhisperCppError::UnsupportedLanguage(value.to_string()))?,
            ),
            None => None,
        };
        if let Some(language) = language.as_ref() {
            let lang_id = unsafe { ffi::whisper_lang_id(language.as_ptr()) };
            if lang_id < 0 {
                return Err(WhisperCppError::UnsupportedLanguage(
                    language.to_string_lossy().into_owned(),
                ));
            }
            params.language = language.as_ptr();
            params.detect_language = false;
        } else {
            params.language = std::ptr::null();
            params.detect_language = true;
        }

        let status = unsafe {
            ffi::whisper_full(context.raw, params, audio.samples.as_ptr(), audio.samples.len() as i32)
        };
        if status != 0 {
            return Err(WhisperCppError::Inference(model_path.display().to_string()));
        }

        let segment_count = unsafe { ffi::whisper_full_n_segments(context.raw) };
        let mut segments = Vec::with_capacity(segment_count.max(0) as usize);
        for index in 0..segment_count {
            let text_ptr = unsafe { ffi::whisper_full_get_segment_text(context.raw, index) };
            let text = c_string(text_ptr)?.trim().to_string();
            let start = unsafe { ffi::whisper_full_get_segment_t0(context.raw, index) };
            let end = unsafe { ffi::whisper_full_get_segment_t1(context.raw, index) };
            let token_count = unsafe { ffi::whisper_full_n_tokens(context.raw, index) };
            let confidence = if token_count > 0 {
                let mut total = 0.0_f32;
                for token_index in 0..token_count {
                    total +=
                        unsafe { ffi::whisper_full_get_token_p(context.raw, index, token_index) };
                }
                Some(total / token_count as f32)
            } else {
                None
            };
            segments.push(WhisperCppSegment {
                index: index as u64,
                start_seconds: Some(timestamp_to_seconds(start)),
                end_seconds: Some(timestamp_to_seconds(end)),
                text,
                confidence,
            });
        }

        let language = unsafe { ffi::whisper_full_lang_id(context.raw) };
        let language = if language >= 0 {
            Some(c_string(unsafe { ffi::whisper_lang_str(language) })?)
        } else {
            None
        };
        let text = join_segments(&segments);

        Ok(WhisperCppTranscription {
            text,
            language,
            segments,
            source: Some(model_path.to_string_lossy().into_owned()),
        })
    }

    fn progress_deref_mut(&mut self) -> Option<&mut ProgressCallback> {
        self.progress
            .as_mut()
            .map(|callback| callback.as_mut() as &mut ProgressCallback)
    }
}

pub fn transcription_catalog() -> WhisperCppCatalog {
    ModelStore::default().catalog()
}

pub fn whisper_cpp_system_info() -> Option<String> {
    let value = unsafe { ffi::whisper_print_system_info() };
    if value.is_null() {
        return None;
    }
    CStr::from_ptr(value)
        .to_str()
        .ok()
        .map(|value| value.to_string())
}

fn emit_progress(
    mut callback: Option<&mut ProgressCallback>,
    phase: WhisperCppPhase,
    message: String,
    progress: Option<f32>,
) {
    if let Some(callback) = callback.as_mut() {
        callback(WhisperCppProgressEvent {
            phase,
            message,
            progress,
        });
    }
}

fn progress_target<'a>(
    external: Option<&'a mut ProgressCallback>,
    internal: Option<&'a mut ProgressCallback>,
) -> Option<&'a mut ProgressCallback> {
    external.or(internal)
}

fn read_wav_mono_f32(path: &Path) -> Result<AudioSamples> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(WhisperCppError::InvalidInput(
            "wav file has no channels".to_string(),
        ));
    }
    if spec.sample_rate != 16_000 {
        return Err(WhisperCppError::InvalidInput(format!(
            "expected 16 kHz wav input, got {} Hz",
            spec.sample_rate
        )));
    }

    let interleaved = match spec.sample_format {
        hound::SampleFormat::Int => read_int_samples(&mut reader, spec.bits_per_sample)?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?,
    };

    let channels = spec.channels as usize;
    let samples = if channels == 1 {
        interleaved
    } else {
        interleaved
            .chunks(channels)
            .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
            .collect()
    };

    Ok(AudioSamples { samples })
}

fn read_int_samples(
    reader: &mut hound::WavReader<std::io::BufReader<File>>,
    bits_per_sample: u16,
) -> Result<Vec<f32>> {
    let scale = ((1_i64 << (bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
    if bits_per_sample <= 16 {
        Ok(reader
            .samples::<i16>()
            .map(|sample| sample.map(|sample| sample as f32 / scale))
            .collect::<std::result::Result<Vec<_>, _>>()?)
    } else {
        Ok(reader
            .samples::<i32>()
            .map(|sample| sample.map(|sample| sample as f32 / scale))
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

fn resolve_threads(value: Option<usize>) -> i32 {
    value
        .or_else(|| thread::available_parallelism().ok().map(usize::from))
        .unwrap_or(4)
        .min(i32::MAX as usize) as i32
}

fn timestamp_to_seconds(value: i64) -> f64 {
    value as f64 / 100.0
}

fn join_segments(segments: &[WhisperCppSegment]) -> Option<String> {
    let text = segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

fn c_string(value: *const std::ffi::c_char) -> Result<String> {
    if value.is_null() {
        return Ok(String::new());
    }
    CStr::from_ptr(value)
        .to_str()
        .map(|value| value.to_string())
        .map_err(|_| WhisperCppError::InvalidUtf8)
}

fn cache_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("VIDEO_ANALYSIS_STUDIO_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(dir).join("video-analysis-studio");
    }
    if cfg!(target_os = "windows") {
        if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(dir).join("video-analysis-studio");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("video-analysis-studio");
    }
    PathBuf::from(".cache/video-analysis-studio")
}

struct AudioSamples {
    samples: Vec<f32>,
}

struct WhisperContext {
    raw: *mut ffi::whisper_context,
}

impl WhisperContext {
    fn from_model(path: &Path) -> Result<Self> {
        let model_path = CString::new(path.to_string_lossy().into_owned())
            .map_err(|_| WhisperCppError::Initialization(path.display().to_string()))?;
        let mut params = unsafe { ffi::whisper_context_default_params() };
        params.use_gpu = cfg!(target_os = "macos");
        params.flash_attn = false;
        let raw = unsafe { ffi::whisper_init_from_file_with_params(model_path.as_ptr(), params) };
        if raw.is_null() {
            return Err(WhisperCppError::Initialization(path.display().to_string()));
        }
        Ok(Self { raw })
    }
}

impl Drop for WhisperContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { ffi::whisper_free(self.raw) };
        }
    }
}

struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: PathBuf) -> Result<Self> {
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            match OpenOptions::new().create_new(true).write(true).open(&path) {
                Ok(mut file) => {
                    let _ = writeln!(file, "{}", std::process::id());
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if Instant::now() >= deadline {
                        return Err(WhisperCppError::Io(error));
                    }
                    thread::sleep(Duration::from_millis(250));
                }
                Err(error) => return Err(WhisperCppError::Io(error)),
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_metadata_matches_expected_file_names() {
        assert_eq!(WhisperCppModel::BaseEn.file_name(), "ggml-base.en.bin");
        assert_eq!(
            WhisperCppModel::LargeV3Turbo.download_url(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin"
        );
    }

    #[test]
    fn catalog_uses_base_en_by_default() {
        let catalog = ModelStore::new(PathBuf::from("/tmp/video-analysis-studio-test")).catalog();
        assert_eq!(catalog.default_model, WhisperCppModel::BaseEn);
        assert_eq!(catalog.models.len(), WhisperCppModel::ALL.len());
    }

    #[test]
    fn cache_paths_are_stable() {
        let store = ModelStore::new(PathBuf::from("/tmp/video-analysis-studio-test"));
        assert_eq!(
            store.model_path(WhisperCppModel::SmallEn),
            PathBuf::from("/tmp/video-analysis-studio-test/models/ggml-small.en.bin")
        );
        assert_eq!(
            store.lock_path(WhisperCppModel::SmallEn),
            PathBuf::from("/tmp/video-analysis-studio-test/models/ggml-small.en.bin.lock")
        );
    }
}
