#![doc = include_str!("../README.md")]

/// A rational number of seconds represented by one timestamp tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timebase {
    /// The numerator of seconds per tick.
    pub num: i32,
    /// The denominator of seconds per tick.
    pub den: i32,
}

impl Timebase {
    /// Creates a timebase from a seconds-per-tick numerator and denominator.
    pub const fn new(num: i32, den: i32) -> Self {
        Self { num, den }
    }

    /// Returns seconds per tick.
    pub fn seconds_per_tick(self) -> f64 {
        self.num as f64 / self.den as f64
    }
}

/// A presentation timestamp paired with its timebase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    /// Presentation timestamp ticks.
    pub pts: i64,
    /// The timebase that gives each tick meaning.
    pub timebase: Timebase,
}

impl Timestamp {
    /// Creates a timestamp.
    pub const fn new(pts: i64, timebase: Timebase) -> Self {
        Self { pts, timebase }
    }

    /// Returns the timestamp in seconds.
    pub fn seconds(self) -> f64 {
        self.pts as f64 * self.timebase.seconds_per_tick()
    }
}

/// A domain-neutral analysis result located optionally in media time.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisEvent {
    /// Timestamp associated with this event.
    pub timestamp: Option<Timestamp>,
    /// The analyzer that produced the event.
    pub analyzer: String,
    /// The event label.
    pub label: String,
    /// Optional confidence or relevance score.
    pub score: Option<f32>,
}

impl AnalysisEvent {
    /// Creates an event without a timestamp or score.
    pub fn new(analyzer: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            timestamp: None,
            analyzer: analyzer.into(),
            label: label.into(),
            score: None,
        }
    }

    /// Adds a timestamp.
    pub fn at_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Adds a score.
    pub fn score(mut self, score: f32) -> Self {
        self.score = Some(score);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{AnalysisEvent, Timebase, Timestamp};

    #[test]
    fn timestamp_uses_its_rational_timebase() {
        let timestamp = Timestamp::new(125, Timebase::new(1, 1_000));
        assert_eq!(timestamp.seconds(), 0.125);
    }

    #[test]
    fn analysis_event_builders_preserve_neutral_contract_data() {
        let timestamp = Timestamp::new(12, Timebase::new(1, 24));
        let event = AnalysisEvent::new("fixture", "cut")
            .at_timestamp(timestamp)
            .score(0.75);

        assert_eq!(event.timestamp, Some(timestamp));
        assert_eq!(event.analyzer, "fixture");
        assert_eq!(event.label, "cut");
        assert_eq!(event.score, Some(0.75));
    }
}
