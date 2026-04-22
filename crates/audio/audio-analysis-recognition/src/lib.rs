use std::collections::BTreeMap;

use audio_analysis_core::{
    interleaved_to_mono, peak, rms, zero_pad_to, ChannelMix, FrameSpec, StreamingFrameBuffer,
    StreamingFrameConfig, WindowFunction,
};
use audio_analysis_fourier::{FourierTransform, Spectrum};
use video_analysis_core::{
    AnalysisEvent, AudioAnalyzer, AudioFrame, DetectError, Result, Timestamp,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AudioEmbedding {
    values: Vec<f32>,
}

impl AudioEmbedding {
    pub fn new(values: impl Into<Vec<f32>>) -> Result<Self> {
        let mut values = values.into();
        normalize_values(&mut values)?;
        Ok(Self { values })
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    pub fn cosine_similarity(&self, other: &Self) -> Result<f32> {
        if self.dimensions() != other.dimensions() {
            return Err(DetectError::InvalidArgument(format!(
                "audio embedding dimensions differ: {} != {}",
                self.dimensions(),
                other.dimensions()
            )));
        }
        Ok(self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(left, right)| left * right)
            .sum())
    }
}

pub trait AudioEmbeddingExtractor {
    fn embed_samples(&self, samples: &[f32], sample_rate: u32) -> Result<AudioEmbedding>;

    fn embed_frame(&self, frame: &AudioFrame<'_>) -> Result<AudioEmbedding> {
        let samples = interleaved_to_mono(frame.data, frame.channels, ChannelMix::Average)?;
        self.embed_samples(&samples, frame.sample_rate)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpectralEmbeddingConfig {
    pub fft_size: usize,
    pub hop_size: usize,
    pub bands: usize,
    pub window: WindowFunction,
}

impl SpectralEmbeddingConfig {
    pub fn new(fft_size: usize, hop_size: usize, bands: usize) -> Result<Self> {
        FourierTransform::new(fft_size)?;
        FrameSpec::new(fft_size, hop_size)?;
        if bands == 0 {
            return Err(DetectError::InvalidArgument(
                "audio embedding bands must be greater than zero".to_string(),
            ));
        }
        Ok(Self {
            fft_size,
            hop_size,
            bands,
            window: WindowFunction::Hann,
        })
    }

    pub fn window(mut self, window: WindowFunction) -> Self {
        self.window = window;
        self
    }
}

impl Default for SpectralEmbeddingConfig {
    fn default() -> Self {
        Self {
            fft_size: 1024,
            hop_size: 512,
            bands: 12,
            window: WindowFunction::Hann,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpectralAudioEmbedder {
    pub config: SpectralEmbeddingConfig,
}

impl SpectralAudioEmbedder {
    pub fn new(config: SpectralEmbeddingConfig) -> Result<Self> {
        SpectralEmbeddingConfig::new(config.fft_size, config.hop_size, config.bands)?;
        Ok(Self { config })
    }

    pub fn streaming_config(&self, channel_mix: ChannelMix) -> Result<StreamingFrameConfig> {
        Ok(
            StreamingFrameConfig::new(self.config.fft_size, self.config.hop_size)?
                .channel_mix(channel_mix)
                .max_buffered_samples(self.config.fft_size + self.config.hop_size),
        )
    }
}

impl AudioEmbeddingExtractor for SpectralAudioEmbedder {
    fn embed_samples(&self, samples: &[f32], sample_rate: u32) -> Result<AudioEmbedding> {
        if sample_rate == 0 {
            return Err(DetectError::InvalidAudioFormat {
                sample_rate,
                channels: 1,
            });
        }
        if samples.is_empty() {
            return Err(DetectError::InvalidArgument(
                "audio samples must not be empty".to_string(),
            ));
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(DetectError::InvalidArgument(
                "audio samples must be finite".to_string(),
            ));
        }

        let transform = FourierTransform::with_window(self.config.fft_size, self.config.window)?;
        let frame_spec = FrameSpec::new(self.config.fft_size, self.config.hop_size)?;
        let frames = if samples.len() < self.config.fft_size {
            vec![zero_pad_to(samples.to_vec(), self.config.fft_size)]
        } else {
            frame_spec
                .frames(samples)
                .map(|(_, frame)| frame.to_vec())
                .collect::<Vec<_>>()
        };

        let mut accumulator = EmbeddingAccumulator::new(self.config.bands);
        for frame in frames {
            let spectrum = transform.analyze_samples(&frame, sample_rate)?;
            accumulator.add_frame(&frame, sample_rate, &spectrum);
        }
        AudioEmbedding::new(accumulator.finish())
    }
}

#[derive(Debug, Clone)]
struct EmbeddingAccumulator {
    bands: usize,
    count: usize,
    sum_rms: f32,
    sum_peak: f32,
    sum_zcr: f32,
    sum_centroid: f32,
    sum_bandwidth: f32,
    sum_rolloff: f32,
    sum_flatness: f32,
    sum_dominant: f32,
    band_energy: Vec<f32>,
}

impl EmbeddingAccumulator {
    fn new(bands: usize) -> Self {
        Self {
            bands,
            count: 0,
            sum_rms: 0.0,
            sum_peak: 0.0,
            sum_zcr: 0.0,
            sum_centroid: 0.0,
            sum_bandwidth: 0.0,
            sum_rolloff: 0.0,
            sum_flatness: 0.0,
            sum_dominant: 0.0,
            band_energy: vec![0.0; bands],
        }
    }

    fn add_frame(&mut self, samples: &[f32], sample_rate: u32, spectrum: &Spectrum) {
        let nyquist = sample_rate as f32 * 0.5;
        let features = spectrum.features();
        self.count += 1;
        self.sum_rms += rms(samples);
        self.sum_peak += peak(samples);
        self.sum_zcr += zero_crossing_rate(samples);
        self.sum_centroid += normalize_frequency(features.centroid_hz, nyquist);
        self.sum_bandwidth += normalize_frequency(features.bandwidth_hz, nyquist);
        self.sum_rolloff += normalize_frequency(features.rolloff_hz, nyquist);
        self.sum_flatness += features.flatness.clamp(0.0, 1.0);
        self.sum_dominant += features
            .dominant_frequency_hz
            .map(|frequency| normalize_frequency(frequency, nyquist))
            .unwrap_or(0.0);
        for (band, energy) in band_energies(spectrum, self.bands).into_iter().enumerate() {
            self.band_energy[band] += energy;
        }
    }

    fn finish(mut self) -> Vec<f32> {
        let count = self.count.max(1) as f32;
        for energy in &mut self.band_energy {
            *energy = (*energy / count).ln_1p();
        }
        let mut values = vec![
            self.sum_rms / count,
            self.sum_peak / count,
            self.sum_zcr / count,
            self.sum_centroid / count,
            self.sum_bandwidth / count,
            self.sum_rolloff / count,
            self.sum_flatness / count,
            self.sum_dominant / count,
        ];
        values.extend(self.band_energy);
        values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioReference {
    id: String,
    label: String,
    embeddings: Vec<AudioEmbedding>,
    attributes: BTreeMap<String, String>,
}

impl AudioReference {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            embeddings: Vec::new(),
            attributes: BTreeMap::new(),
        }
    }

    pub fn with_embedding(mut self, embedding: AudioEmbedding) -> Result<Self> {
        self.add_embedding(embedding)?;
        Ok(self)
    }

    pub fn with_samples<E: AudioEmbeddingExtractor>(
        mut self,
        samples: &[f32],
        sample_rate: u32,
        extractor: &E,
    ) -> Result<Self> {
        self.add_samples(samples, sample_rate, extractor)?;
        Ok(self)
    }

    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn add_embedding(&mut self, embedding: AudioEmbedding) -> Result<()> {
        if let Some(dimensions) = self.dimensions() {
            if dimensions != embedding.dimensions() {
                return Err(DetectError::InvalidArgument(format!(
                    "audio reference `{}` has mixed embedding dimensions: {} != {}",
                    self.id,
                    dimensions,
                    embedding.dimensions()
                )));
            }
        }
        self.embeddings.push(embedding);
        Ok(())
    }

    pub fn add_samples<E: AudioEmbeddingExtractor>(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        extractor: &E,
    ) -> Result<()> {
        self.add_embedding(extractor.embed_samples(samples, sample_rate)?)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn embeddings(&self) -> &[AudioEmbedding] {
        &self.embeddings
    }

    pub fn attributes(&self) -> &BTreeMap<String, String> {
        &self.attributes
    }

    pub fn dimensions(&self) -> Option<usize> {
        self.embeddings.first().map(AudioEmbedding::dimensions)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioReferenceLibrary {
    references: BTreeMap<String, AudioReference>,
    dimensions: Option<usize>,
}

impl AudioReferenceLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_reference(&mut self, reference: AudioReference) -> Result<()> {
        if reference.id().is_empty() {
            return Err(DetectError::InvalidArgument(
                "audio reference id must not be empty".to_string(),
            ));
        }
        if reference.label().is_empty() {
            return Err(DetectError::InvalidArgument(
                "audio reference label must not be empty".to_string(),
            ));
        }
        let dimensions = reference.dimensions().ok_or_else(|| {
            DetectError::InvalidArgument(format!(
                "audio reference `{}` must contain at least one embedding",
                reference.id()
            ))
        })?;
        self.validate_dimensions(dimensions)?;
        if self.references.contains_key(reference.id()) {
            return Err(DetectError::InvalidArgument(
                "duplicate audio reference id".to_string(),
            ));
        }
        self.dimensions = Some(dimensions);
        self.references
            .insert(reference.id().to_string(), reference);
        Ok(())
    }

    pub fn add_reference_embedding(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        embedding: AudioEmbedding,
    ) -> Result<()> {
        self.add_reference(AudioReference::new(id, label).with_embedding(embedding)?)
    }

    pub fn add_reference_samples<E: AudioEmbeddingExtractor>(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        samples: &[f32],
        sample_rate: u32,
        extractor: &E,
    ) -> Result<()> {
        self.add_reference(AudioReference::new(id, label).with_samples(
            samples,
            sample_rate,
            extractor,
        )?)
    }

    pub fn add_sample(&mut self, id: &str, embedding: AudioEmbedding) -> Result<()> {
        self.validate_dimensions(embedding.dimensions())?;
        let reference = self.references.get_mut(id).ok_or_else(|| {
            DetectError::InvalidArgument(format!("unknown audio reference id `{id}`"))
        })?;
        reference.add_embedding(embedding)
    }

    pub fn add_sample_from_samples<E: AudioEmbeddingExtractor>(
        &mut self,
        id: &str,
        samples: &[f32],
        sample_rate: u32,
        extractor: &E,
    ) -> Result<()> {
        self.add_sample(id, extractor.embed_samples(samples, sample_rate)?)
    }

    pub fn reference(&self, id: &str) -> Option<&AudioReference> {
        self.references.get(id)
    }

    pub fn references(&self) -> impl Iterator<Item = &AudioReference> {
        self.references.values()
    }

    pub fn len(&self) -> usize {
        self.references.len()
    }

    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    pub fn dimensions(&self) -> Option<usize> {
        self.dimensions
    }

    pub fn search(
        &self,
        embedding: &AudioEmbedding,
        options: &AudioMatchOptions,
    ) -> Result<Vec<AudioRecognitionMatch>> {
        options.validate()?;
        self.validate_dimensions(embedding.dimensions())?;
        let mut matches = Vec::new();
        for reference in self.references.values() {
            let Some(score) = best_reference_score(reference, embedding)? else {
                continue;
            };
            if score < options.min_score {
                continue;
            }
            matches.push(AudioRecognitionMatch {
                reference_id: reference.id().to_string(),
                label: reference.label().to_string(),
                score,
                attributes: reference.attributes().clone(),
            });
        }
        matches.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.reference_id.cmp(&right.reference_id))
        });
        matches.truncate(options.max_results);
        Ok(matches)
    }

    fn validate_dimensions(&self, dimensions: usize) -> Result<()> {
        if let Some(expected) = self.dimensions {
            if expected != dimensions {
                return Err(DetectError::InvalidArgument(format!(
                    "audio embedding dimensions differ from reference library: {dimensions} != {expected}"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioMatchOptions {
    pub min_score: f32,
    pub max_results: usize,
}

impl AudioMatchOptions {
    pub fn new(min_score: f32) -> Result<Self> {
        let options = Self {
            min_score,
            ..Self::default()
        };
        options.validate()?;
        Ok(options)
    }

    pub fn max_results(mut self, value: usize) -> Self {
        self.max_results = value;
        self
    }

    fn validate(&self) -> Result<()> {
        if !self.min_score.is_finite() || !(-1.0..=1.0).contains(&self.min_score) {
            return Err(DetectError::InvalidArgument(
                "minimum audio recognition score must be finite and between -1.0 and 1.0"
                    .to_string(),
            ));
        }
        if self.max_results == 0 {
            return Err(DetectError::InvalidArgument(
                "max audio recognition results must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for AudioMatchOptions {
    fn default() -> Self {
        Self {
            min_score: 0.85,
            max_results: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioRecognitionMatch {
    pub reference_id: String,
    pub label: String,
    pub score: f32,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioSimilarity {
    pub score: f32,
}

pub fn compare_audio_samples<E: AudioEmbeddingExtractor>(
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    extractor: &E,
) -> Result<AudioSimilarity> {
    let left = extractor.embed_samples(left, sample_rate)?;
    let right = extractor.embed_samples(right, sample_rate)?;
    Ok(AudioSimilarity {
        score: left.cosine_similarity(&right)?,
    })
}

pub struct AudioRecognitionAnalyzer<E> {
    name: String,
    extractor: E,
    library: AudioReferenceLibrary,
    match_options: AudioMatchOptions,
    windows: StreamingFrameBuffer,
}

impl<E: AudioEmbeddingExtractor> AudioRecognitionAnalyzer<E> {
    pub fn new(
        name: impl Into<String>,
        extractor: E,
        library: AudioReferenceLibrary,
        streaming_config: StreamingFrameConfig,
    ) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            extractor,
            library,
            match_options: AudioMatchOptions::default(),
            windows: StreamingFrameBuffer::new(streaming_config)?,
        })
    }

    pub fn match_options(mut self, options: AudioMatchOptions) -> Self {
        self.match_options = options;
        self
    }

    pub fn library(&self) -> &AudioReferenceLibrary {
        &self.library
    }

    pub fn extractor(&self) -> &E {
        &self.extractor
    }

    pub fn reset(&mut self) {
        self.windows.reset();
    }
}

impl AudioRecognitionAnalyzer<SpectralAudioEmbedder> {
    pub fn spectral(name: impl Into<String>, library: AudioReferenceLibrary) -> Result<Self> {
        let extractor = SpectralAudioEmbedder::default();
        let streaming_config = extractor.streaming_config(ChannelMix::Average)?;
        Self::new(name, extractor, library, streaming_config)
    }
}

impl<E: AudioEmbeddingExtractor> AudioAnalyzer for AudioRecognitionAnalyzer<E> {
    fn name(&self) -> &str {
        &self.name
    }

    fn process_frame(&mut self, frame: &AudioFrame<'_>) -> Result<Vec<AnalysisEvent>> {
        let windows = self.windows.push_frame(frame)?;
        let mut events = Vec::new();
        for window in windows {
            let embedding = self
                .extractor
                .embed_samples(&window.samples, window.sample_rate)?;
            for recognition_match in self.library.search(&embedding, &self.match_options)? {
                events.push(event_for_match(
                    self.name(),
                    window.timestamp,
                    recognition_match,
                ));
            }
        }
        Ok(events)
    }
}

fn best_reference_score(
    reference: &AudioReference,
    embedding: &AudioEmbedding,
) -> Result<Option<f32>> {
    let mut best = None;
    for reference_embedding in reference.embeddings() {
        let score = reference_embedding.cosine_similarity(embedding)?;
        best = Some(best.map(|value: f32| value.max(score)).unwrap_or(score));
    }
    Ok(best)
}

fn event_for_match(
    analyzer: &str,
    timestamp: Timestamp,
    recognition_match: AudioRecognitionMatch,
) -> AnalysisEvent {
    AnalysisEvent::new(
        analyzer,
        format!(
            "audio:recognized:{}:{}",
            recognition_match.reference_id, recognition_match.label
        ),
    )
    .at_timestamp(timestamp)
    .score(recognition_match.score)
}

fn normalize_values(values: &mut [f32]) -> Result<()> {
    if values.is_empty() {
        return Err(DetectError::InvalidArgument(
            "audio embedding must not be empty".to_string(),
        ));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(DetectError::InvalidArgument(
            "audio embedding values must be finite".to_string(),
        ));
    }
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON || !norm.is_finite() {
        return Err(DetectError::InvalidArgument(
            "audio embedding norm must be finite and non-zero".to_string(),
        ));
    }
    for value in values {
        *value /= norm;
    }
    Ok(())
}

fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let crossings = samples
        .windows(2)
        .filter(|pair| (pair[0] >= 0.0 && pair[1] < 0.0) || (pair[0] < 0.0 && pair[1] >= 0.0))
        .count();
    crossings as f32 / (samples.len() - 1) as f32
}

fn normalize_frequency(frequency_hz: f32, nyquist: f32) -> f32 {
    if nyquist <= f32::EPSILON {
        0.0
    } else {
        (frequency_hz / nyquist).clamp(0.0, 1.0)
    }
}

fn band_energies(spectrum: &Spectrum, bands: usize) -> Vec<f32> {
    let mut energies = vec![0.0_f32; bands];
    if spectrum.bins.len() <= 1 {
        return energies;
    }
    let max_index = spectrum.bins.len() - 1;
    for bin in spectrum.bins.iter().skip(1) {
        let band = ((bin.index - 1) * bands / max_index).min(bands - 1);
        energies[band] += bin.power;
    }
    energies
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_analysis_test_support::assert_approx_eq;
    use video_analysis_core::{AudioBuffer, OwnedAudioFrame, Timebase};

    fn sine(freq_hz: f32, sample_rate: u32, seconds: f32) -> Vec<f32> {
        let samples = (sample_rate as f32 * seconds) as usize;
        (0..samples)
            .map(|index| {
                let t = index as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * freq_hz * t).sin()
            })
            .collect()
    }

    fn embedder() -> SpectralAudioEmbedder {
        SpectralAudioEmbedder::new(SpectralEmbeddingConfig::new(512, 256, 8).unwrap()).unwrap()
    }

    #[test]
    fn embedding_validation_rejects_empty_non_finite_and_mismatched_dimensions() {
        assert!(AudioEmbedding::new(Vec::<f32>::new()).is_err());
        assert!(AudioEmbedding::new(vec![f32::NAN]).is_err());
        assert!(AudioEmbedding::new(vec![0.0, 0.0]).is_err());
        let left = AudioEmbedding::new(vec![1.0, 0.0]).unwrap();
        let right = AudioEmbedding::new(vec![1.0, 0.0, 0.0]).unwrap();
        assert!(left.cosine_similarity(&right).is_err());
    }

    #[test]
    fn spectral_embedding_config_validates_dimensions() {
        assert!(SpectralEmbeddingConfig::new(0, 256, 8).is_err());
        assert!(SpectralEmbeddingConfig::new(500, 256, 8).is_err());
        assert!(SpectralEmbeddingConfig::new(512, 0, 8).is_err());
        assert!(SpectralEmbeddingConfig::new(512, 256, 0).is_err());
    }

    #[test]
    fn embeds_samples_with_stable_dimensions() {
        let embedding = embedder()
            .embed_samples(&sine(440.0, 8_000, 0.5), 8_000)
            .unwrap();

        assert_eq!(embedding.dimensions(), 16);
        let norm = embedding
            .values()
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        assert_approx_eq(norm, 1.0, 1.0e-5);
        assert!(embedding.values().iter().all(|value| value.is_finite()));
        assert!(embedder().embed_samples(&[], 8_000).is_err());
        assert!(embedder().embed_samples(&[f32::INFINITY], 8_000).is_err());
        assert!(embedder().embed_samples(&[0.0; 512], 0).is_err());
    }

    #[test]
    fn compares_similar_audio_above_different_audio() {
        let extractor = embedder();
        let a = sine(440.0, 8_000, 0.5);
        let b = sine(445.0, 8_000, 0.5);
        let c = sine(1_600.0, 8_000, 0.5);

        let similar = compare_audio_samples(&a, &b, 8_000, &extractor).unwrap();
        let different = compare_audio_samples(&a, &c, 8_000, &extractor).unwrap();

        assert!(similar.score > different.score);
    }

    #[test]
    fn reference_library_searches_nearest_sample() {
        let extractor = embedder();
        let mut library = AudioReferenceLibrary::new();
        library
            .add_reference_samples(
                "low",
                "Low tone",
                &sine(220.0, 8_000, 0.5),
                8_000,
                &extractor,
            )
            .unwrap();
        library
            .add_reference_samples(
                "high",
                "High tone",
                &sine(1_600.0, 8_000, 0.5),
                8_000,
                &extractor,
            )
            .unwrap();
        let query = extractor
            .embed_samples(&sine(225.0, 8_000, 0.5), 8_000)
            .unwrap();

        let matches = library
            .search(&query, &AudioMatchOptions::default())
            .unwrap();

        assert_eq!(matches[0].reference_id, "low");
    }

    #[test]
    fn reference_library_rejects_duplicate_and_mixed_dimensions() {
        let mut library = AudioReferenceLibrary::new();
        library
            .add_reference_embedding("a", "A", AudioEmbedding::new(vec![1.0, 0.0]).unwrap())
            .unwrap();
        assert!(library
            .add_reference_embedding(
                "a",
                "Duplicate",
                AudioEmbedding::new(vec![1.0, 0.0]).unwrap()
            )
            .is_err());
        assert!(library
            .add_reference_embedding("b", "B", AudioEmbedding::new(vec![1.0, 0.0, 0.0]).unwrap())
            .is_err());
        let mut reference = AudioReference::new("mixed", "Mixed")
            .with_embedding(AudioEmbedding::new(vec![1.0, 0.0]).unwrap())
            .unwrap();
        assert!(reference
            .add_embedding(AudioEmbedding::new(vec![1.0, 0.0, 0.0]).unwrap())
            .is_err());
    }

    #[test]
    fn match_options_validate_score_range_and_result_count() {
        assert!(AudioMatchOptions::new(-1.1).is_err());
        assert!(AudioMatchOptions::new(1.1).is_err());
        assert!(AudioMatchOptions::new(f32::NAN).is_err());

        let mut library = AudioReferenceLibrary::new();
        library
            .add_reference_embedding("a", "A", AudioEmbedding::new(vec![1.0, 0.0]).unwrap())
            .unwrap();
        let query = AudioEmbedding::new(vec![1.0, 0.0]).unwrap();
        assert!(library
            .search(
                &query,
                &AudioMatchOptions {
                    min_score: -1.0,
                    max_results: 0,
                },
            )
            .is_err());
    }

    #[test]
    fn search_order_is_deterministic_for_equal_scores() {
        let mut library = AudioReferenceLibrary::new();
        library
            .add_reference_embedding("b", "B", AudioEmbedding::new(vec![1.0, 0.0]).unwrap())
            .unwrap();
        library
            .add_reference_embedding("a", "A", AudioEmbedding::new(vec![1.0, 0.0]).unwrap())
            .unwrap();
        let matches = library
            .search(
                &AudioEmbedding::new(vec![1.0, 0.0]).unwrap(),
                &AudioMatchOptions {
                    min_score: -1.0,
                    max_results: 2,
                },
            )
            .unwrap();
        assert_eq!(matches[0].reference_id, "a");
        assert_eq!(matches[1].reference_id, "b");
    }

    #[test]
    fn adding_extra_sample_improves_match() {
        let extractor = embedder();
        let mut library = AudioReferenceLibrary::new();
        library
            .add_reference_samples("tone", "Tone", &sine(220.0, 8_000, 0.5), 8_000, &extractor)
            .unwrap();
        let query = extractor
            .embed_samples(&sine(440.0, 8_000, 0.5), 8_000)
            .unwrap();
        let before = library
            .search(
                &query,
                &AudioMatchOptions {
                    min_score: -1.0,
                    max_results: 1,
                },
            )
            .unwrap()[0]
            .score;

        library
            .add_sample_from_samples("tone", &sine(440.0, 8_000, 0.5), 8_000, &extractor)
            .unwrap();
        let after = library
            .search(
                &query,
                &AudioMatchOptions {
                    min_score: -1.0,
                    max_results: 1,
                },
            )
            .unwrap()[0]
            .score;

        assert!(after > before);
    }

    #[test]
    fn analyzer_emits_recognition_events_from_streaming_windows() {
        let extractor = embedder();
        let mut library = AudioReferenceLibrary::new();
        let reference = AudioReference::new("a4", "A4")
            .attribute("source", "synthetic")
            .with_samples(&sine(440.0, 8_000, 0.5), 8_000, &extractor)
            .unwrap();
        library.add_reference(reference).unwrap();
        let config = StreamingFrameConfig::new(512, 512).unwrap();
        let mut analyzer =
            AudioRecognitionAnalyzer::new("audio_identity", extractor, library, config).unwrap();
        let samples = AudioBuffer::F32(sine(440.0, 8_000, 0.5));
        let frame = OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 8_000)),
            8_000,
            1,
            samples,
        )
        .unwrap();

        let events = analyzer.process_frame(&frame.as_frame().unwrap()).unwrap();

        assert!(!events.is_empty());
        assert_eq!(events[0].analyzer, "audio_identity");
        assert_eq!(
            events[0].timestamp,
            Some(Timestamp::new(0, Timebase::new(1, 8_000)))
        );
        assert!(events[0].score.unwrap() >= 0.85);
        assert!(events[0].label.starts_with("audio:recognized:a4:A4"));
        assert_eq!(
            analyzer
                .library()
                .reference("a4")
                .unwrap()
                .attributes()
                .get("source"),
            Some(&"synthetic".to_string())
        );
    }

    #[test]
    fn analyzer_reset_clears_streaming_window_state() {
        let extractor = embedder();
        let mut library = AudioReferenceLibrary::new();
        library
            .add_reference_samples("a4", "A4", &sine(440.0, 8_000, 0.5), 8_000, &extractor)
            .unwrap();
        let config = StreamingFrameConfig::new(512, 512).unwrap();
        let mut analyzer =
            AudioRecognitionAnalyzer::new("audio_identity", extractor, library, config).unwrap();
        let frame = OwnedAudioFrame::new(
            Timestamp::new(0, Timebase::new(1, 8_000)),
            8_000,
            1,
            AudioBuffer::F32(sine(440.0, 8_000, 0.5)),
        )
        .unwrap();

        assert!(!analyzer
            .process_frame(&frame.as_frame().unwrap())
            .unwrap()
            .is_empty());
        analyzer.reset();
        assert!(!analyzer
            .process_frame(&frame.as_frame().unwrap())
            .unwrap()
            .is_empty());
    }
}
