#![doc = include_str!("../README.md")]

use std::collections::{BTreeMap, VecDeque};
use std::f32::consts::PI;

use video_analysis_core::{
    Cut, DetectError, FramePosition, MetricsSink, Result, SceneDetector, VideoFrame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashFilterMode {
    Merge,
    Suppress,
}

#[derive(Debug, Clone)]
pub struct FlashFilter {
    mode: FlashFilterMode,
    length: u64,
    last_above: Option<u64>,
    merge_enabled: bool,
    merge_triggered: bool,
    merge_start: Option<u64>,
}

impl FlashFilter {
    pub fn new(mode: FlashFilterMode, length: u64) -> Self {
        Self {
            mode,
            length,
            last_above: None,
            merge_enabled: false,
            merge_triggered: false,
            merge_start: None,
        }
    }

    pub fn max_behind(&self) -> usize {
        match self.mode {
            FlashFilterMode::Suppress => 0,
            FlashFilterMode::Merge => self.length as usize,
        }
    }

    pub fn filter(&mut self, frame_index: u64, above_threshold: bool) -> Vec<u64> {
        if self.length == 0 {
            return above_threshold.then_some(frame_index).into_iter().collect();
        }
        if self.last_above.is_none() {
            self.last_above = Some(frame_index);
        }
        match self.mode {
            FlashFilterMode::Suppress => self.filter_suppress(frame_index, above_threshold),
            FlashFilterMode::Merge => self.filter_merge(frame_index, above_threshold),
        }
    }

    fn filter_suppress(&mut self, frame_index: u64, above_threshold: bool) -> Vec<u64> {
        let min_length_met = frame_index.saturating_sub(self.last_above.unwrap()) >= self.length;
        if above_threshold && min_length_met {
            self.last_above = Some(frame_index);
            vec![frame_index]
        } else {
            Vec::new()
        }
    }

    fn filter_merge(&mut self, frame_index: u64, above_threshold: bool) -> Vec<u64> {
        let min_length_met = frame_index.saturating_sub(self.last_above.unwrap()) >= self.length;
        if above_threshold {
            self.last_above = Some(frame_index);
        }
        if self.merge_triggered {
            let merged = self
                .last_above
                .unwrap()
                .saturating_sub(self.merge_start.unwrap_or(frame_index));
            if min_length_met && !above_threshold && merged >= self.length {
                self.merge_triggered = false;
                return vec![self.last_above.unwrap()];
            }
            return Vec::new();
        }
        if !above_threshold {
            return Vec::new();
        }
        if min_length_met {
            self.merge_enabled = true;
            return vec![frame_index];
        }
        if self.merge_enabled {
            self.merge_triggered = true;
            self.merge_start = Some(frame_index);
        }
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlgorithmScore {
    pub position: FramePosition,
    pub raw: f32,
    pub normalized: f32,
}

pub trait ScoreAlgorithm {
    fn name(&self) -> &'static str;

    fn latency(&self) -> usize {
        0
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<AlgorithmScore>>;

    fn finish(
        &mut self,
        _last_position: FramePosition,
        _metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<AlgorithmScore>> {
        Ok(Vec::new())
    }
}

pub struct WeightedComponent {
    algorithm: Box<dyn ScoreAlgorithm>,
    weight: f32,
}

impl WeightedComponent {
    pub fn new<A: ScoreAlgorithm + 'static>(algorithm: A, weight: f32) -> Result<Self> {
        if !weight.is_finite() || weight <= 0.0 {
            return Err(DetectError::InvalidArgument(
                "component weight must be finite and greater than zero".to_string(),
            ));
        }
        Ok(Self {
            algorithm: Box::new(algorithm),
            weight,
        })
    }

    pub fn name(&self) -> &'static str {
        self.algorithm.name()
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }
}

pub struct WeightedCompositeDetector {
    components: Vec<WeightedComponent>,
    threshold: f32,
    flash_filter: FlashFilter,
    max_latency: usize,
    pending: BTreeMap<u64, PendingCompositeScore>,
}

#[derive(Debug, Clone)]
struct PendingCompositeScore {
    position: FramePosition,
    scores: Vec<Option<AlgorithmScore>>,
}

impl PendingCompositeScore {
    fn new(position: FramePosition, len: usize) -> Self {
        Self {
            position,
            scores: vec![None; len],
        }
    }

    fn is_complete(&self) -> bool {
        self.scores.iter().all(Option::is_some)
    }
}

#[derive(Default)]
pub struct WeightedCompositeDetectorBuilder {
    components: Vec<WeightedComponent>,
    threshold: Option<f32>,
    min_scene_len: Option<u64>,
    filter_mode: Option<FlashFilterMode>,
}

impl WeightedCompositeDetector {
    pub const METRIC_KEYS: &'static [&'static str] = &[
        "combined_score",
        "combined_cut",
        "combined_weight_sum",
        "combined_vote_count",
    ];

    pub fn builder() -> WeightedCompositeDetectorBuilder {
        WeightedCompositeDetectorBuilder::default()
    }

    fn record_score(&mut self, index: usize, score: AlgorithmScore) {
        let component_count = self.components.len();
        self.pending
            .entry(score.position.frame_index)
            .or_insert_with(|| PendingCompositeScore::new(score.position, component_count))
            .scores[index] = Some(score);
    }

    fn flush_ready(
        &mut self,
        max_frame_index: u64,
        mut metrics: Option<&mut dyn MetricsSink>,
    ) -> Vec<Cut> {
        let ready: Vec<u64> = self
            .pending
            .iter()
            .take_while(|(frame_index, _)| **frame_index <= max_frame_index)
            .map(|(frame_index, _)| *frame_index)
            .collect();
        let mut cuts = Vec::new();
        for frame_index in ready {
            let Some(pending) = self.pending.remove(&frame_index) else {
                continue;
            };
            if !pending.is_complete() {
                continue;
            }
            let component_scores: Vec<AlgorithmScore> =
                pending.scores.into_iter().flatten().collect();
            let combined = self.combined_score(&component_scores);
            let cut_frames = self
                .flash_filter
                .filter(pending.position.frame_index, combined >= self.threshold);
            if let Some(metrics) = metrics.as_mut() {
                record_combined_metrics(
                    &mut **metrics,
                    pending.position,
                    &self.components,
                    &component_scores,
                    combined,
                    &cut_frames,
                );
            }
            cuts.extend(cut_frames.into_iter().map(|frame_index| Cut {
                position: position_like(pending.position, frame_index),
                detector: self.name(),
                score: Some(combined),
            }));
        }
        cuts
    }

    fn combined_score(&self, scores: &[AlgorithmScore]) -> f32 {
        let mut weighted = 0.0;
        let mut total_weight = 0.0;
        for (component, score) in self.components.iter().zip(scores) {
            weighted += score.normalized * component.weight;
            total_weight += component.weight;
        }
        if total_weight == 0.0 {
            0.0
        } else {
            weighted / total_weight
        }
    }
}

impl WeightedCompositeDetectorBuilder {
    pub fn component<A: ScoreAlgorithm + 'static>(mut self, algorithm: A, weight: f32) -> Self {
        self.components.push(WeightedComponent {
            algorithm: Box::new(algorithm),
            weight,
        });
        self
    }

    pub fn weighted_component(mut self, component: WeightedComponent) -> Self {
        self.components.push(component);
        self
    }

    pub fn threshold(mut self, value: f32) -> Self {
        self.threshold = Some(value);
        self
    }

    pub fn min_scene_len(mut self, value: u64) -> Self {
        self.min_scene_len = Some(value);
        self
    }

    pub fn filter_mode(mut self, value: FlashFilterMode) -> Self {
        self.filter_mode = Some(value);
        self
    }

    pub fn build(self) -> Result<WeightedCompositeDetector> {
        if self.components.is_empty() {
            return Err(DetectError::InvalidArgument(
                "at least one composite component is required".to_string(),
            ));
        }
        for component in &self.components {
            if !component.weight.is_finite() || component.weight <= 0.0 {
                return Err(DetectError::InvalidArgument(format!(
                    "component `{}` weight must be finite and greater than zero",
                    component.name()
                )));
            }
        }
        let max_latency = self
            .components
            .iter()
            .map(|component| component.algorithm.latency())
            .max()
            .unwrap_or(0);
        Ok(WeightedCompositeDetector {
            components: self.components,
            threshold: self.threshold.unwrap_or(0.5),
            flash_filter: FlashFilter::new(
                self.filter_mode.unwrap_or(FlashFilterMode::Merge),
                self.min_scene_len.unwrap_or(15),
            ),
            max_latency,
            pending: BTreeMap::new(),
        })
    }
}

impl SceneDetector for WeightedCompositeDetector {
    fn name(&self) -> &'static str {
        "combined"
    }

    fn metric_keys(&self) -> &'static [&'static str] {
        Self::METRIC_KEYS
    }

    fn event_buffer_len(&self) -> usize {
        self.max_latency + self.flash_filter.max_behind()
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        mut metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        for index in 0..self.components.len() {
            let scores = match metrics.as_mut() {
                Some(metrics) => self.components[index]
                    .algorithm
                    .process_frame(frame, Some(&mut **metrics))?,
                None => self.components[index]
                    .algorithm
                    .process_frame(frame, None)?,
            };
            for score in scores {
                self.record_score(index, score);
            }
        }
        if frame.position.frame_index < self.max_latency as u64 {
            Ok(Vec::new())
        } else {
            let max_ready = frame.position.frame_index - self.max_latency as u64;
            Ok(self.flush_ready(max_ready, metrics))
        }
    }

    fn finish(
        &mut self,
        last_position: FramePosition,
        mut metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        for index in 0..self.components.len() {
            let scores = match metrics.as_mut() {
                Some(metrics) => self.components[index]
                    .algorithm
                    .finish(last_position, Some(&mut **metrics))?,
                None => self.components[index]
                    .algorithm
                    .finish(last_position, None)?,
            };
            for score in scores {
                self.record_score(index, score);
            }
        }
        Ok(self.flush_ready(last_position.frame_index, metrics))
    }
}

fn record_combined_metrics(
    metrics: &mut dyn MetricsSink,
    position: FramePosition,
    components: &[WeightedComponent],
    scores: &[AlgorithmScore],
    combined: f32,
    cut_frames: &[u64],
) {
    let frame_index = position.frame_index;
    let weight_sum: f32 = components.iter().map(WeightedComponent::weight).sum();
    let vote_count = scores
        .iter()
        .filter(|score| score.normalized >= 0.5)
        .count();
    metrics.set_metric(frame_index, "combined_score", combined as f64);
    metrics.set_metric(frame_index, "combined_cut", 0.0);
    metrics.set_metric(frame_index, "combined_weight_sum", weight_sum as f64);
    metrics.set_metric(frame_index, "combined_vote_count", vote_count as f64);
    for (component, score) in components.iter().zip(scores) {
        let prefix = format!("combined.{}", component.name());
        metrics.set_metric(frame_index, &format!("{prefix}.raw"), score.raw as f64);
        metrics.set_metric(
            frame_index,
            &format!("{prefix}.normalized"),
            score.normalized as f64,
        );
        metrics.set_metric(
            frame_index,
            &format!("{prefix}.weighted"),
            (score.normalized * component.weight()) as f64,
        );
    }
    for cut_frame in cut_frames {
        metrics.set_metric(*cut_frame, "combined_cut", 1.0);
    }
}

fn normalize_threshold(raw: f32, threshold: f32) -> f32 {
    if threshold <= 0.0 {
        if raw > 0.0 {
            1.0
        } else {
            0.0
        }
    } else {
        (raw / (2.0 * threshold)).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContentWeights {
    pub delta_hue: f32,
    pub delta_sat: f32,
    pub delta_lum: f32,
    pub delta_edges: f32,
}

impl Default for ContentWeights {
    fn default() -> Self {
        Self {
            delta_hue: 1.0,
            delta_sat: 1.0,
            delta_lum: 1.0,
            delta_edges: 0.0,
        }
    }
}

impl ContentWeights {
    pub const LUMA_ONLY: Self = Self {
        delta_hue: 0.0,
        delta_sat: 0.0,
        delta_lum: 1.0,
        delta_edges: 0.0,
    };

    fn total(self) -> f32 {
        self.delta_hue.abs() + self.delta_sat.abs() + self.delta_lum.abs() + self.delta_edges.abs()
    }
}

#[derive(Debug, Clone)]
pub struct ContentDetector {
    threshold: f32,
    scorer: ContentScorer,
    flash_filter: FlashFilter,
}

impl Default for ContentDetector {
    fn default() -> Self {
        Self::new(27.0, 15)
    }
}

impl ContentDetector {
    pub const METRIC_KEYS: &'static [&'static str] = &[
        "content_val",
        "delta_hue",
        "delta_sat",
        "delta_lum",
        "delta_edges",
    ];

    pub fn new(threshold: f32, min_scene_len: u64) -> Self {
        Self {
            threshold,
            scorer: ContentScorer::new(ContentWeights::default(), None),
            flash_filter: FlashFilter::new(FlashFilterMode::Merge, min_scene_len),
        }
    }

    pub fn with_weights(mut self, weights: ContentWeights) -> Self {
        self.scorer.weights = weights;
        self
    }

    pub fn luma_only(mut self, value: bool) -> Self {
        if value {
            self.scorer.weights = ContentWeights::LUMA_ONLY;
        }
        self
    }

    pub fn kernel_size(mut self, value: Option<usize>) -> Result<Self> {
        if let Some(size) = value {
            if size < 3 || size % 2 == 0 {
                return Err(DetectError::InvalidArgument(
                    "kernel_size must be an odd integer >= 3".to_string(),
                ));
            }
        }
        self.scorer.kernel_size = value;
        Ok(self)
    }

    pub fn filter_mode(mut self, mode: FlashFilterMode, min_scene_len: u64) -> Self {
        self.flash_filter = FlashFilter::new(mode, min_scene_len);
        self
    }
}

impl SceneDetector for ContentDetector {
    fn name(&self) -> &'static str {
        "content"
    }

    fn metric_keys(&self) -> &'static [&'static str] {
        Self::METRIC_KEYS
    }

    fn event_buffer_len(&self) -> usize {
        self.flash_filter.max_behind()
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        let score = self.scorer.score(frame, metrics)?;
        let cut_frames = self
            .flash_filter
            .filter(frame.position.frame_index, score >= self.threshold);
        Ok(cut_frames
            .into_iter()
            .map(|frame_index| Cut {
                position: position_like(frame.position, frame_index),
                detector: self.name(),
                score: Some(score),
            })
            .collect())
    }
}

#[derive(Debug, Clone)]
struct ContentScorer {
    weights: ContentWeights,
    kernel_size: Option<usize>,
    last: Option<FrameData>,
}

impl ContentScorer {
    fn new(weights: ContentWeights, kernel_size: Option<usize>) -> Self {
        Self {
            weights,
            kernel_size,
            last: None,
        }
    }

    fn score(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<f32> {
        let calculate_edges = self.weights.delta_edges > 0.0;
        let current = FrameData::from_frame(frame, calculate_edges, self.kernel_size)?;
        let Some(previous) = self.last.replace(current.clone()) else {
            if let Some(metrics) = metrics {
                set_content_metrics(
                    metrics,
                    frame.position.frame_index,
                    0.0,
                    (0.0, 0.0, 0.0, 0.0),
                );
            }
            return Ok(0.0);
        };

        let dh = mean_abs_diff(&current.hue, &previous.hue);
        let ds = mean_abs_diff(&current.sat, &previous.sat);
        let dl = mean_abs_diff(&current.lum, &previous.lum);
        let de = match (&current.edges, &previous.edges) {
            (Some(left), Some(right)) => mean_abs_diff(left, right),
            _ => 0.0,
        };
        let total_weight = self.weights.total();
        let score = if total_weight == 0.0 {
            0.0
        } else {
            ((dh * self.weights.delta_hue)
                + (ds * self.weights.delta_sat)
                + (dl * self.weights.delta_lum)
                + (de * self.weights.delta_edges))
                / total_weight
        };
        if let Some(metrics) = metrics {
            set_content_metrics(metrics, frame.position.frame_index, score, (dh, ds, dl, de));
        }
        Ok(score)
    }
}

fn set_content_metrics(
    metrics: &mut dyn MetricsSink,
    frame_index: u64,
    score: f32,
    components: (f32, f32, f32, f32),
) {
    metrics.set_metric(frame_index, "content_val", score as f64);
    metrics.set_metric(frame_index, "delta_hue", components.0 as f64);
    metrics.set_metric(frame_index, "delta_sat", components.1 as f64);
    metrics.set_metric(frame_index, "delta_lum", components.2 as f64);
    metrics.set_metric(frame_index, "delta_edges", components.3 as f64);
}

#[derive(Debug, Clone)]
pub struct ContentScoreAlgorithm {
    threshold: f32,
    scorer: ContentScorer,
}

impl ContentScoreAlgorithm {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            scorer: ContentScorer::new(ContentWeights::default(), None),
        }
    }

    pub fn with_weights(mut self, weights: ContentWeights) -> Self {
        self.scorer.weights = weights;
        self
    }

    pub fn luma_only(mut self, value: bool) -> Self {
        if value {
            self.scorer.weights = ContentWeights::LUMA_ONLY;
        }
        self
    }

    pub fn kernel_size(mut self, value: Option<usize>) -> Result<Self> {
        if let Some(size) = value {
            if size < 3 || size % 2 == 0 {
                return Err(DetectError::InvalidArgument(
                    "kernel_size must be an odd integer >= 3".to_string(),
                ));
            }
        }
        self.scorer.kernel_size = value;
        Ok(self)
    }
}

impl ScoreAlgorithm for ContentScoreAlgorithm {
    fn name(&self) -> &'static str {
        "content"
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<AlgorithmScore>> {
        let raw = self.scorer.score(frame, metrics)?;
        Ok(vec![AlgorithmScore {
            position: frame.position,
            raw,
            normalized: normalize_threshold(raw, self.threshold),
        }])
    }
}

#[derive(Debug, Clone)]
struct FrameData {
    hue: Vec<u8>,
    sat: Vec<u8>,
    lum: Vec<u8>,
    edges: Option<Vec<u8>>,
}

impl FrameData {
    fn from_frame(
        frame: &VideoFrame<'_>,
        calculate_edges: bool,
        kernel_size: Option<usize>,
    ) -> Result<Self> {
        let pixels = frame.pixel_count();
        let mut hue = Vec::with_capacity(pixels);
        let mut sat = Vec::with_capacity(pixels);
        let mut lum = Vec::with_capacity(pixels);
        for y in 0..frame.height {
            for x in 0..frame.width {
                let [r, g, b] = frame.pixel_rgb(x, y);
                let (h, s, v) = rgb_to_hsv(r, g, b);
                hue.push(h);
                sat.push(s);
                lum.push(v);
            }
        }
        let edges =
            calculate_edges.then(|| detect_edges(&lum, frame.width, frame.height, kernel_size));
        Ok(Self {
            hue,
            sat,
            lum,
            edges,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveDetector {
    scorer: ContentScorer,
    adaptive_threshold: f32,
    min_scene_len: u64,
    window_width: usize,
    min_content_val: f32,
    last_cut: Option<u64>,
    buffer: VecDeque<(FramePosition, f32)>,
}

impl Default for AdaptiveDetector {
    fn default() -> Self {
        Self::new(3.0, 15, 2, 15.0)
    }
}

impl AdaptiveDetector {
    pub const METRIC_KEYS: &'static [&'static str] = &[
        "content_val",
        "delta_hue",
        "delta_sat",
        "delta_lum",
        "delta_edges",
        "adaptive_ratio",
    ];

    pub fn new(
        adaptive_threshold: f32,
        min_scene_len: u64,
        window_width: usize,
        min_content_val: f32,
    ) -> Self {
        assert!(window_width >= 1, "window_width must be at least 1");
        Self {
            scorer: ContentScorer::new(ContentWeights::default(), None),
            adaptive_threshold,
            min_scene_len,
            window_width,
            min_content_val,
            last_cut: None,
            buffer: VecDeque::new(),
        }
    }

    pub fn luma_only(mut self, value: bool) -> Self {
        if value {
            self.scorer.weights = ContentWeights::LUMA_ONLY;
        }
        self
    }
}

impl SceneDetector for AdaptiveDetector {
    fn name(&self) -> &'static str {
        "adaptive"
    }

    fn metric_keys(&self) -> &'static [&'static str] {
        Self::METRIC_KEYS
    }

    fn event_buffer_len(&self) -> usize {
        self.window_width
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        mut metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        let score = match metrics.as_mut() {
            Some(metrics) => self.scorer.score(frame, Some(&mut **metrics))?,
            None => self.scorer.score(frame, None)?,
        };
        if self.last_cut.is_none() {
            self.last_cut = Some(frame.position.frame_index);
        }
        let required = 1 + self.window_width * 2;
        self.buffer.push_back((frame.position, score));
        if self.buffer.len() < required {
            return Ok(Vec::new());
        }
        while self.buffer.len() > required {
            self.buffer.pop_front();
        }

        let target = self.buffer[self.window_width];
        let neighbor_sum: f32 = self
            .buffer
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != self.window_width)
            .map(|(_, (_, score))| *score)
            .sum();
        let average = neighbor_sum / (2.0 * self.window_width as f32);
        let ratio = if average.abs() < 0.00001 {
            if target.1 >= self.min_content_val {
                255.0
            } else {
                0.0
            }
        } else {
            (target.1 / average).min(255.0)
        };
        if let Some(metrics) = metrics.as_mut() {
            (**metrics).set_metric(target.0.frame_index, "adaptive_ratio", ratio as f64);
        }
        let threshold_met = ratio >= self.adaptive_threshold && target.1 >= self.min_content_val;
        let min_length_met = frame
            .position
            .frame_index
            .saturating_sub(self.last_cut.unwrap())
            >= self.min_scene_len;
        if threshold_met && min_length_met {
            self.last_cut = Some(target.0.frame_index);
            Ok(vec![Cut {
                position: target.0,
                detector: self.name(),
                score: Some(target.1),
            }])
        } else {
            Ok(Vec::new())
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveScoreAlgorithm {
    scorer: ContentScorer,
    adaptive_threshold: f32,
    window_width: usize,
    min_content_val: f32,
    buffer: VecDeque<(FramePosition, f32)>,
}

impl AdaptiveScoreAlgorithm {
    pub fn new(adaptive_threshold: f32, window_width: usize, min_content_val: f32) -> Self {
        assert!(window_width >= 1, "window_width must be at least 1");
        Self {
            scorer: ContentScorer::new(ContentWeights::default(), None),
            adaptive_threshold,
            window_width,
            min_content_val,
            buffer: VecDeque::new(),
        }
    }

    pub fn luma_only(mut self, value: bool) -> Self {
        if value {
            self.scorer.weights = ContentWeights::LUMA_ONLY;
        }
        self
    }
}

impl ScoreAlgorithm for AdaptiveScoreAlgorithm {
    fn name(&self) -> &'static str {
        "adaptive"
    }

    fn latency(&self) -> usize {
        self.window_width
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        mut metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<AlgorithmScore>> {
        let content_score = match metrics.as_mut() {
            Some(metrics) => self.scorer.score(frame, Some(&mut **metrics))?,
            None => self.scorer.score(frame, None)?,
        };
        let required = 1 + self.window_width * 2;
        self.buffer.push_back((frame.position, content_score));
        if self.buffer.len() < required {
            return Ok(Vec::new());
        }
        while self.buffer.len() > required {
            self.buffer.pop_front();
        }

        let target = self.buffer[self.window_width];
        let neighbor_sum: f32 = self
            .buffer
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != self.window_width)
            .map(|(_, (_, score))| *score)
            .sum();
        let average = neighbor_sum / (2.0 * self.window_width as f32);
        let ratio = if average.abs() < 0.00001 {
            if target.1 >= self.min_content_val {
                255.0
            } else {
                0.0
            }
        } else {
            (target.1 / average).min(255.0)
        };
        if let Some(metrics) = metrics.as_mut() {
            (**metrics).set_metric(target.0.frame_index, "adaptive_ratio", ratio as f64);
        }
        let normalized = if target.1 >= self.min_content_val {
            normalize_threshold(ratio, self.adaptive_threshold)
        } else {
            0.0
        };
        Ok(vec![AlgorithmScore {
            position: target.0,
            raw: ratio,
            normalized,
        }])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdMethod {
    Floor,
    Ceiling,
}

#[derive(Debug, Clone)]
pub struct ThresholdDetector {
    threshold: f32,
    min_scene_len: u64,
    fade_bias: f32,
    add_final_scene: bool,
    method: ThresholdMethod,
    processed_frame: bool,
    last_scene_cut: Option<u64>,
    last_fade_frame: u64,
    last_fade_type: Option<FadeType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FadeType {
    In,
    Out,
}

impl Default for ThresholdDetector {
    fn default() -> Self {
        Self::new(12.0, 15)
    }
}

impl ThresholdDetector {
    pub const METRIC_KEYS: &'static [&'static str] = &["average_rgb"];

    pub fn new(threshold: f32, min_scene_len: u64) -> Self {
        Self {
            threshold,
            min_scene_len,
            fade_bias: 0.0,
            add_final_scene: false,
            method: ThresholdMethod::Floor,
            processed_frame: false,
            last_scene_cut: None,
            last_fade_frame: 0,
            last_fade_type: None,
        }
    }

    pub fn fade_bias(mut self, value: f32) -> Self {
        self.fade_bias = value.clamp(-1.0, 1.0);
        self
    }

    pub fn add_final_scene(mut self, value: bool) -> Self {
        self.add_final_scene = value;
        self
    }

    pub fn method(mut self, value: ThresholdMethod) -> Self {
        self.method = value;
        self
    }

    fn is_out(&self, average: f32) -> bool {
        match self.method {
            ThresholdMethod::Floor => average < self.threshold,
            ThresholdMethod::Ceiling => average >= self.threshold,
        }
    }

    fn is_in(&self, average: f32) -> bool {
        !self.is_out(average)
    }
}

impl SceneDetector for ThresholdDetector {
    fn name(&self) -> &'static str {
        "threshold"
    }

    fn metric_keys(&self) -> &'static [&'static str] {
        Self::METRIC_KEYS
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        if self.last_scene_cut.is_none() {
            self.last_scene_cut = Some(frame.position.frame_index);
        }
        let average = mean_rgb(frame)?;
        if let Some(metrics) = metrics {
            metrics.set_metric(frame.position.frame_index, "average_rgb", average as f64);
        }

        let mut cuts = Vec::new();
        if self.processed_frame {
            if self.last_fade_type == Some(FadeType::In) && self.is_out(average) {
                self.last_fade_type = Some(FadeType::Out);
                self.last_fade_frame = frame.position.frame_index;
            } else if self.last_fade_type == Some(FadeType::Out) && self.is_in(average) {
                if frame
                    .position
                    .frame_index
                    .saturating_sub(self.last_scene_cut.unwrap())
                    >= self.min_scene_len
                {
                    let f_out = self.last_fade_frame as f32;
                    let f_in = frame.position.frame_index as f32;
                    let split = ((f_in + f_out + self.fade_bias * (f_in - f_out)) / 2.0) as u64;
                    cuts.push(Cut {
                        position: position_like(frame.position, split),
                        detector: self.name(),
                        score: Some(average),
                    });
                    self.last_scene_cut = Some(frame.position.frame_index);
                }
                self.last_fade_type = Some(FadeType::In);
                self.last_fade_frame = frame.position.frame_index;
            }
        } else {
            self.last_fade_frame = 0;
            self.last_fade_type = Some(if self.is_out(average) {
                FadeType::Out
            } else {
                FadeType::In
            });
        }
        self.processed_frame = true;
        Ok(cuts)
    }

    fn finish(
        &mut self,
        last_position: FramePosition,
        _metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        let min_length_met = self
            .last_scene_cut
            .map(|cut| last_position.frame_index.saturating_sub(cut) >= self.min_scene_len)
            .unwrap_or(last_position.frame_index >= self.min_scene_len);
        if self.last_fade_type == Some(FadeType::Out) && self.add_final_scene && min_length_met {
            Ok(vec![Cut {
                position: position_like(last_position, self.last_fade_frame),
                detector: self.name(),
                score: None,
            }])
        } else {
            Ok(Vec::new())
        }
    }
}

#[derive(Debug, Clone)]
pub struct HistogramDetector {
    threshold: f32,
    bins: usize,
    min_scene_len: u64,
    last_hist: Option<Vec<f32>>,
    last_scene_cut: Option<u64>,
}

impl Default for HistogramDetector {
    fn default() -> Self {
        Self::new(0.05, 256, 15)
    }
}

impl HistogramDetector {
    pub const METRIC_KEYS: &'static [&'static str] = &["hist_diff"];

    pub fn new(threshold: f32, bins: usize, min_scene_len: u64) -> Self {
        Self {
            threshold: 1.0 - threshold.clamp(0.0, 1.0),
            bins,
            min_scene_len,
            last_hist: None,
            last_scene_cut: None,
        }
    }
}

impl SceneDetector for HistogramDetector {
    fn name(&self) -> &'static str {
        "histogram"
    }

    fn metric_keys(&self) -> &'static [&'static str] {
        Self::METRIC_KEYS
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        if self.last_scene_cut.is_none() {
            self.last_scene_cut = Some(frame.position.frame_index);
        }
        let hist = luma_histogram(frame, self.bins)?;
        let mut cuts = Vec::new();
        if let Some(last_hist) = &self.last_hist {
            let correlation = histogram_correlation(last_hist, &hist);
            if let Some(metrics) = metrics {
                metrics.set_metric(frame.position.frame_index, "hist_diff", correlation as f64);
            }
            if correlation <= self.threshold
                && frame
                    .position
                    .frame_index
                    .saturating_sub(self.last_scene_cut.unwrap())
                    >= self.min_scene_len
            {
                self.last_scene_cut = Some(frame.position.frame_index);
                cuts.push(Cut {
                    position: frame.position,
                    detector: self.name(),
                    score: Some(correlation),
                });
            }
        }
        self.last_hist = Some(hist);
        Ok(cuts)
    }
}

#[derive(Debug, Clone)]
pub struct HistogramScoreAlgorithm {
    threshold: f32,
    bins: usize,
    last_hist: Option<Vec<f32>>,
}

impl HistogramScoreAlgorithm {
    pub fn new(threshold: f32, bins: usize) -> Self {
        Self {
            threshold,
            bins,
            last_hist: None,
        }
    }
}

impl ScoreAlgorithm for HistogramScoreAlgorithm {
    fn name(&self) -> &'static str {
        "histogram"
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<AlgorithmScore>> {
        let hist = luma_histogram(frame, self.bins)?;
        let raw = if let Some(last_hist) = &self.last_hist {
            let correlation = histogram_correlation(last_hist, &hist);
            if let Some(metrics) = metrics {
                metrics.set_metric(frame.position.frame_index, "hist_diff", correlation as f64);
            }
            1.0 - correlation
        } else {
            if let Some(metrics) = metrics {
                metrics.set_metric(frame.position.frame_index, "hist_diff", 1.0);
            }
            0.0
        };
        self.last_hist = Some(hist);
        Ok(vec![AlgorithmScore {
            position: frame.position,
            raw,
            normalized: normalize_threshold(raw, self.threshold),
        }])
    }
}

#[derive(Debug, Clone)]
pub struct HashDetector {
    threshold: f32,
    size: usize,
    lowpass: usize,
    min_scene_len: u64,
    last_hash: Option<Vec<bool>>,
    last_scene_cut: Option<u64>,
}

impl Default for HashDetector {
    fn default() -> Self {
        Self::new(0.395, 16, 2, 15)
    }
}

impl HashDetector {
    pub const METRIC_KEYS: &'static [&'static str] = &["hash_dist"];

    pub fn new(threshold: f32, size: usize, lowpass: usize, min_scene_len: u64) -> Self {
        Self {
            threshold,
            size,
            lowpass: lowpass.max(1),
            min_scene_len,
            last_hash: None,
            last_scene_cut: None,
        }
    }
}

impl SceneDetector for HashDetector {
    fn name(&self) -> &'static str {
        "hash"
    }

    fn metric_keys(&self) -> &'static [&'static str] {
        Self::METRIC_KEYS
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<Cut>> {
        if self.last_scene_cut.is_none() {
            self.last_scene_cut = Some(frame.position.frame_index);
        }
        let hash = phash(frame, self.size, self.lowpass)?;
        let mut cuts = Vec::new();
        if let Some(last_hash) = &self.last_hash {
            let dist = hash
                .iter()
                .zip(last_hash.iter())
                .filter(|(left, right)| left != right)
                .count() as f32
                / (self.size * self.size) as f32;
            if let Some(metrics) = metrics {
                metrics.set_metric(frame.position.frame_index, "hash_dist", dist as f64);
            }
            if dist >= self.threshold
                && frame
                    .position
                    .frame_index
                    .saturating_sub(self.last_scene_cut.unwrap())
                    >= self.min_scene_len
            {
                self.last_scene_cut = Some(frame.position.frame_index);
                cuts.push(Cut {
                    position: frame.position,
                    detector: self.name(),
                    score: Some(dist),
                });
            }
        }
        self.last_hash = Some(hash);
        Ok(cuts)
    }
}

#[derive(Debug, Clone)]
pub struct HashScoreAlgorithm {
    threshold: f32,
    size: usize,
    lowpass: usize,
    last_hash: Option<Vec<bool>>,
}

impl HashScoreAlgorithm {
    pub fn new(threshold: f32, size: usize, lowpass: usize) -> Self {
        Self {
            threshold,
            size,
            lowpass: lowpass.max(1),
            last_hash: None,
        }
    }
}

impl ScoreAlgorithm for HashScoreAlgorithm {
    fn name(&self) -> &'static str {
        "hash"
    }

    fn process_frame(
        &mut self,
        frame: &VideoFrame<'_>,
        metrics: Option<&mut dyn MetricsSink>,
    ) -> Result<Vec<AlgorithmScore>> {
        let hash = phash(frame, self.size, self.lowpass)?;
        let raw = if let Some(last_hash) = &self.last_hash {
            let dist = hash
                .iter()
                .zip(last_hash.iter())
                .filter(|(left, right)| left != right)
                .count() as f32
                / (self.size * self.size) as f32;
            if let Some(metrics) = metrics {
                metrics.set_metric(frame.position.frame_index, "hash_dist", dist as f64);
            }
            dist
        } else {
            if let Some(metrics) = metrics {
                metrics.set_metric(frame.position.frame_index, "hash_dist", 0.0);
            }
            0.0
        };
        self.last_hash = Some(hash);
        Ok(vec![AlgorithmScore {
            position: frame.position,
            raw,
            normalized: normalize_threshold(raw, self.threshold),
        }])
    }
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    let sat = if max == 0.0 { 0.0 } else { delta / max };
    (
        (hue / 360.0 * 255.0) as u8,
        (sat * 255.0) as u8,
        (max * 255.0) as u8,
    )
}

fn mean_abs_diff(left: &[u8], right: &[u8]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (*left as i16 - *right as i16).unsigned_abs() as u64)
        .sum::<u64>() as f32
        / left.len() as f32
}

fn detect_edges(lum: &[u8], width: u32, height: u32, kernel_size: Option<usize>) -> Vec<u8> {
    let width = width as usize;
    let height = height as usize;
    if width < 3 || height < 3 {
        return vec![0; width * height];
    }
    let mut edges = vec![0_u8; width * height];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let i = |x: usize, y: usize| lum[y * width + x] as i32;
            let gx = -i(x - 1, y - 1) + i(x + 1, y - 1) - 2 * i(x - 1, y) + 2 * i(x + 1, y)
                - i(x - 1, y + 1)
                + i(x + 1, y + 1);
            let gy = -i(x - 1, y - 1) - 2 * i(x, y - 1) - i(x + 1, y - 1)
                + i(x - 1, y + 1)
                + 2 * i(x, y + 1)
                + i(x + 1, y + 1);
            let mag = ((gx * gx + gy * gy) as f32).sqrt();
            edges[y * width + x] = if mag > 64.0 { 255 } else { 0 };
        }
    }
    dilate(&edges, width, height, kernel_size.unwrap_or(3))
}

fn dilate(input: &[u8], width: usize, height: usize, kernel_size: usize) -> Vec<u8> {
    let radius = kernel_size / 2;
    let mut output = vec![0_u8; input.len()];
    for y in 0..height {
        for x in 0..width {
            let y0 = y.saturating_sub(radius);
            let y1 = (y + radius).min(height - 1);
            let x0 = x.saturating_sub(radius);
            let x1 = (x + radius).min(width - 1);
            let mut value = 0;
            for yy in y0..=y1 {
                for xx in x0..=x1 {
                    value = value.max(input[yy * width + xx]);
                }
            }
            output[y * width + x] = value;
        }
    }
    output
}

fn mean_rgb(frame: &VideoFrame<'_>) -> Result<f32> {
    let mut total = 0_u64;
    for y in 0..frame.height {
        for x in 0..frame.width {
            let [r, g, b] = frame.pixel_rgb(x, y);
            total += r as u64 + g as u64 + b as u64;
        }
    }
    Ok(total as f32 / (frame.pixel_count() * 3) as f32)
}

fn luma_histogram(frame: &VideoFrame<'_>, bins: usize) -> Result<Vec<f32>> {
    if bins == 0 {
        return Err(DetectError::InvalidArgument(
            "histogram bins must be greater than zero".to_string(),
        ));
    }
    let mut hist = vec![0_f32; bins];
    for y in 0..frame.height {
        for x in 0..frame.width {
            let [r, g, b] = frame.pixel_rgb(x, y);
            let y = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32)
                .round()
                .clamp(0.0, 255.0) as usize;
            let bin = (y * bins / 256).min(bins - 1);
            hist[bin] += 1.0;
        }
    }
    let total = frame.pixel_count() as f32;
    for value in &mut hist {
        *value /= total;
    }
    Ok(hist)
}

fn histogram_correlation(left: &[f32], right: &[f32]) -> f32 {
    let mean_left = left.iter().sum::<f32>() / left.len() as f32;
    let mean_right = right.iter().sum::<f32>() / right.len() as f32;
    let mut numerator = 0.0;
    let mut denom_left = 0.0;
    let mut denom_right = 0.0;
    for (left, right) in left.iter().zip(right.iter()) {
        let dl = left - mean_left;
        let dr = right - mean_right;
        numerator += dl * dr;
        denom_left += dl * dl;
        denom_right += dr * dr;
    }
    if denom_left == 0.0 || denom_right == 0.0 {
        if left == right {
            1.0
        } else {
            0.0
        }
    } else {
        numerator / (denom_left.sqrt() * denom_right.sqrt())
    }
}

fn phash(frame: &VideoFrame<'_>, size: usize, lowpass: usize) -> Result<Vec<bool>> {
    let image_size = size * lowpass;
    let gray = resize_luma_nearest(frame, image_size, image_size)?;
    let max_value = gray.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let normalized: Vec<f32> = gray.into_iter().map(|value| value / max_value).collect();
    let mut coeffs = vec![0_f32; size * size];
    for v in 0..size {
        for u in 0..size {
            let mut sum = 0.0;
            for y in 0..image_size {
                for x in 0..image_size {
                    let pixel = normalized[y * image_size + x];
                    let cu = ((2 * x + 1) as f32 * u as f32 * PI / (2.0 * image_size as f32)).cos();
                    let cv = ((2 * y + 1) as f32 * v as f32 * PI / (2.0 * image_size as f32)).cos();
                    sum += pixel * cu * cv;
                }
            }
            coeffs[v * size + u] = sum;
        }
    }
    let mut sorted = coeffs.clone();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let median = sorted[sorted.len() / 2];
    Ok(coeffs.into_iter().map(|value| value > median).collect())
}

fn resize_luma_nearest(frame: &VideoFrame<'_>, width: usize, height: usize) -> Result<Vec<f32>> {
    let mut output = vec![0_f32; width * height];
    for y in 0..height {
        let src_y = y as u32 * frame.height / height as u32;
        for x in 0..width {
            let src_x = x as u32 * frame.width / width as u32;
            let [r, g, b] = frame.pixel_rgb(src_x, src_y);
            output[y * width + x] = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        }
    }
    Ok(output)
}

fn position_like(current: FramePosition, frame_index: u64) -> FramePosition {
    let delta = frame_index as i64 - current.frame_index as i64;
    FramePosition {
        frame_index,
        timestamp: video_analysis_core::Timestamp::new(
            current.timestamp.pts + delta,
            current.timestamp.timebase,
        ),
    }
}

#[cfg(test)]
mod tests {
    use num_rational::Rational64;
    use video_analysis_core::{FramePosition, MetricsStore, VideoFrame};

    use super::*;

    fn frame(frame_index: u64, rgb: [u8; 3]) -> video_analysis_core::OwnedVideoFrame {
        let mut data = Vec::new();
        for _ in 0..16 {
            data.extend_from_slice(&rgb);
        }
        video_analysis_core::OwnedVideoFrame {
            position: FramePosition::from_frame_index(frame_index, Rational64::new(30, 1)),
            width: 4,
            height: 4,
            pixel_format: video_analysis_core::PixelFormat::Rgb24,
            data,
            stride: 12,
        }
    }

    struct FixedScoreAlgorithm {
        name: &'static str,
        normalized: f32,
    }

    impl ScoreAlgorithm for FixedScoreAlgorithm {
        fn name(&self) -> &'static str {
            self.name
        }

        fn process_frame(
            &mut self,
            frame: &VideoFrame<'_>,
            _metrics: Option<&mut dyn MetricsSink>,
        ) -> Result<Vec<AlgorithmScore>> {
            Ok(vec![AlgorithmScore {
                position: frame.position,
                raw: self.normalized,
                normalized: self.normalized,
            }])
        }
    }

    struct DelayedScoreAlgorithm {
        name: &'static str,
        latency: usize,
        normalized: f32,
    }

    impl ScoreAlgorithm for DelayedScoreAlgorithm {
        fn name(&self) -> &'static str {
            self.name
        }

        fn latency(&self) -> usize {
            self.latency
        }

        fn process_frame(
            &mut self,
            frame: &VideoFrame<'_>,
            _metrics: Option<&mut dyn MetricsSink>,
        ) -> Result<Vec<AlgorithmScore>> {
            let latency = self.latency as u64;
            if frame.position.frame_index < latency {
                return Ok(Vec::new());
            }
            Ok(vec![AlgorithmScore {
                position: position_like(frame.position, frame.position.frame_index - latency),
                raw: self.normalized,
                normalized: self.normalized,
            }])
        }
    }

    #[test]
    fn flash_filter_suppresses_until_min_length() {
        let mut filter = FlashFilter::new(FlashFilterMode::Suppress, 3);
        assert!(filter.filter(0, true).is_empty());
        assert!(filter.filter(2, true).is_empty());
        assert_eq!(filter.filter(3, true), vec![3]);
    }

    #[test]
    fn content_detector_finds_hard_cut() {
        let mut detector = ContentDetector::new(10.0, 1);
        let mut metrics = MetricsStore::default();
        let first = frame(0, [0, 0, 0]);
        let second = frame(1, [255, 255, 255]);
        assert!(detector
            .process_frame(&first.as_frame(), Some(&mut metrics))
            .unwrap()
            .is_empty());
        let cuts = detector
            .process_frame(&second.as_frame(), Some(&mut metrics))
            .unwrap();
        assert_eq!(cuts.len(), 1);
        assert!(metrics.get(1, "content_val").unwrap() > 10.0);
    }

    #[test]
    fn threshold_detector_finds_fade_out_in_split() {
        let mut detector = ThresholdDetector::new(12.0, 1);
        let frames = [
            frame(0, [255, 255, 255]),
            frame(1, [0, 0, 0]),
            frame(2, [255, 255, 255]),
        ];
        let mut cuts = Vec::new();
        for frame in frames {
            cuts.extend(detector.process_frame(&frame.as_frame(), None).unwrap());
        }
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].position.frame_index, 1);
    }

    #[test]
    fn histogram_detector_records_metric() {
        let mut detector = HistogramDetector::new(0.05, 8, 1);
        let mut metrics = MetricsStore::default();
        let first = frame(0, [0, 0, 0]);
        let second = frame(1, [255, 255, 255]);
        detector
            .process_frame(&first.as_frame(), Some(&mut metrics))
            .unwrap();
        detector
            .process_frame(&second.as_frame(), Some(&mut metrics))
            .unwrap();
        assert!(metrics.get(1, "hist_diff").is_some());
    }

    #[test]
    fn hash_detector_processes_frames() {
        let mut detector = HashDetector::new(0.0, 4, 2, 1);
        let first = frame(0, [0, 0, 0]);
        let second = frame(1, [255, 255, 255]);
        assert!(detector
            .process_frame(&first.as_frame(), None)
            .unwrap()
            .is_empty());
        let cuts = detector.process_frame(&second.as_frame(), None).unwrap();
        assert_eq!(cuts.len(), 1);
    }

    #[test]
    fn content_fixture_no_cut_for_low_motion() {
        let mut detector = ContentDetector::new(20.0, 1);
        let frames = [
            frame(0, [20, 20, 20]),
            frame(1, [24, 24, 24]),
            frame(2, [28, 28, 28]),
            frame(3, [32, 32, 32]),
        ];
        let mut cuts = Vec::new();
        for frame in frames {
            cuts.extend(detector.process_frame(&frame.as_frame(), None).unwrap());
        }

        assert!(cuts.is_empty());
    }

    #[test]
    fn content_fixture_flash_is_suppressed() {
        let mut detector = ContentDetector::new(10.0, 3).filter_mode(FlashFilterMode::Suppress, 3);
        let frames = [
            frame(0, [0, 0, 0]),
            frame(1, [255, 255, 255]),
            frame(2, [0, 0, 0]),
        ];
        let mut cuts = Vec::new();
        for frame in frames {
            cuts.extend(detector.process_frame(&frame.as_frame(), None).unwrap());
        }

        assert!(cuts.is_empty());
    }

    #[test]
    fn content_fixture_dissolve_stays_below_hard_cut_threshold() {
        let mut detector = ContentDetector::new(80.0, 1).luma_only(true);
        let frames = [
            frame(0, [0, 0, 0]),
            frame(1, [32, 32, 32]),
            frame(2, [64, 64, 64]),
            frame(3, [96, 96, 96]),
            frame(4, [128, 128, 128]),
            frame(5, [160, 160, 160]),
        ];
        let mut cuts = Vec::new();
        for frame in frames {
            cuts.extend(detector.process_frame(&frame.as_frame(), None).unwrap());
        }

        assert!(cuts.is_empty());
    }

    #[test]
    fn frame_validation_rejects_short_buffer() {
        let pos = FramePosition::from_frame_index(0, Rational64::new(30, 1));
        assert!(VideoFrame::rgb24(pos, 4, 4, &[0; 4]).is_err());
    }

    #[test]
    fn normalization_maps_threshold_to_half() {
        assert_eq!(normalize_threshold(10.0, 10.0), 0.5);
        assert_eq!(normalize_threshold(20.0, 10.0), 1.0);
        assert_eq!(normalize_threshold(1.0, 0.0), 1.0);
        assert_eq!(normalize_threshold(0.0, 0.0), 0.0);
    }

    #[test]
    fn composite_detector_ignores_weak_combined_score() {
        let mut detector = WeightedCompositeDetector::builder()
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "content",
                        normalized: 0.4,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "hash",
                        normalized: 0.4,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .threshold(0.5)
            .min_scene_len(0)
            .build()
            .unwrap();

        let cuts = detector
            .process_frame(&frame(0, [0, 0, 0]).as_frame(), None)
            .unwrap();
        assert!(cuts.is_empty());
    }

    #[test]
    fn composite_detector_emits_cut_from_combined_score() {
        let mut detector = WeightedCompositeDetector::builder()
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "content",
                        normalized: 0.4,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "hash",
                        normalized: 0.6,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .threshold(0.5)
            .min_scene_len(0)
            .build()
            .unwrap();

        let cuts = detector
            .process_frame(&frame(0, [0, 0, 0]).as_frame(), None)
            .unwrap();
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].detector, "combined");
        assert_eq!(cuts[0].score, Some(0.5));
    }

    #[test]
    fn composite_detector_records_provenance_metrics() {
        let mut detector = WeightedCompositeDetector::builder()
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "content",
                        normalized: 0.5,
                    },
                    2.0,
                )
                .unwrap(),
            )
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "hash",
                        normalized: 1.0,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .threshold(0.5)
            .min_scene_len(0)
            .build()
            .unwrap();
        let mut metrics = MetricsStore::default();

        detector
            .process_frame(&frame(0, [0, 0, 0]).as_frame(), Some(&mut metrics))
            .unwrap();

        let combined = metrics.get(0, "combined_score").unwrap();
        assert!((combined - (2.0 / 3.0)).abs() < 0.000001);
        assert_eq!(metrics.get(0, "combined_vote_count"), Some(2.0));
        assert_eq!(metrics.get(0, "combined.content.raw"), Some(0.5));
        assert_eq!(metrics.get(0, "combined.content.weighted"), Some(1.0));
        assert_eq!(metrics.get(0, "combined.hash.normalized"), Some(1.0));
    }

    #[test]
    fn composite_detector_waits_for_latency_before_evaluating() {
        let mut detector = WeightedCompositeDetector::builder()
            .weighted_component(
                WeightedComponent::new(
                    FixedScoreAlgorithm {
                        name: "fast",
                        normalized: 1.0,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .weighted_component(
                WeightedComponent::new(
                    DelayedScoreAlgorithm {
                        name: "slow",
                        latency: 2,
                        normalized: 1.0,
                    },
                    1.0,
                )
                .unwrap(),
            )
            .threshold(0.5)
            .min_scene_len(0)
            .build()
            .unwrap();

        assert!(detector
            .process_frame(&frame(0, [0, 0, 0]).as_frame(), None)
            .unwrap()
            .is_empty());
        assert!(detector
            .process_frame(&frame(1, [0, 0, 0]).as_frame(), None)
            .unwrap()
            .is_empty());
        let cuts = detector
            .process_frame(&frame(2, [0, 0, 0]).as_frame(), None)
            .unwrap();
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].position.frame_index, 0);
    }
}
