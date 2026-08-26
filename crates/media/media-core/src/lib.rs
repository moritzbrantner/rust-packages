#![doc = include_str!("../README.md")]

use std::cmp::Ordering;

/// A compact identifier for the pixel layout of a video frame.
///
/// This is neutral stream-format metadata. Pixel buffers and video frames
/// remain owned by the visual domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// Packed RGB with eight bits per channel.
    Rgb24,
    /// Packed BGR with eight bits per channel.
    Bgr24,
}

/// A compact identifier for the scalar representation of audio samples.
///
/// This is neutral stream-format metadata. Audio buffers and frames remain
/// owned by the audio domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSampleFormat {
    /// Unsigned eight-bit samples.
    U8,
    /// Signed 16-bit samples.
    I16,
    /// Signed 32-bit samples.
    I32,
    /// 32-bit floating-point samples.
    F32,
}

/// Errors shared by media contract consumers.
#[derive(Debug)]
pub enum DetectError {
    /// The pixel format is unsupported by the requested operation.
    UnsupportedPixelFormat(PixelFormat),
    /// The audio sample format is unsupported by the requested operation.
    UnsupportedAudioSampleFormat(AudioSampleFormat),
    /// A frame buffer is shorter than its declared dimensions require.
    InvalidFrameBuffer {
        /// Minimum required byte length.
        expected: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Media dimensions are invalid.
    InvalidDimensions {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// Audio format metadata is invalid.
    InvalidAudioFormat {
        /// Sample rate in hertz.
        sample_rate: u32,
        /// Number of audio channels.
        channels: u16,
    },
    /// A media source failed.
    Source(String),
    /// An argument failed validation.
    InvalidArgument(String),
    /// An I/O operation failed.
    Io(std::io::Error),
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPixelFormat(format) => {
                write!(f, "unsupported pixel format: {format:?}")
            }
            Self::UnsupportedAudioSampleFormat(format) => {
                write!(f, "unsupported audio sample format: {format:?}")
            }
            Self::InvalidFrameBuffer { expected, actual } => {
                write!(
                    f,
                    "invalid frame buffer: expected at least {expected} bytes, got {actual}"
                )
            }
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid dimensions: {width}x{height}")
            }
            Self::InvalidAudioFormat {
                sample_rate,
                channels,
            } => {
                write!(
                    f,
                    "invalid audio format: sample_rate={sample_rate}, channels={channels}"
                )
            }
            Self::Source(message) => write!(f, "video source error: {message}"),
            Self::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
        }
    }
}

impl std::error::Error for DetectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DetectError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Result type shared by media contract consumers.
pub type Result<T> = std::result::Result<T, DetectError>;

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
    ///
    /// This compatibility constructor does not validate its arguments. New
    /// boundary code should prefer [`Self::try_new`].
    pub const fn new(num: i32, den: i32) -> Self {
        Self { num, den }
    }

    /// Creates a validated positive timebase.
    pub fn try_new(num: i32, den: i32) -> Result<Self> {
        let value = Self { num, den };
        value.validate()?;
        Ok(value)
    }

    /// Validates that one tick represents a finite positive duration.
    pub fn validate(self) -> Result<()> {
        if self.num <= 0 || self.den <= 0 {
            return Err(DetectError::InvalidArgument(format!(
                "timebase numerator and denominator must be positive, got {}/{}",
                self.num, self.den
            )));
        }
        Ok(())
    }

    /// Returns seconds per tick.
    pub fn seconds_per_tick(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Returns seconds per tick after validating the timebase.
    pub fn checked_seconds_per_tick(self) -> Result<f64> {
        self.validate()?;
        Ok(self.seconds_per_tick())
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
    ///
    /// This compatibility constructor does not validate its timebase. New
    /// boundary code should prefer [`Self::try_new`].
    pub const fn new(pts: i64, timebase: Timebase) -> Self {
        Self { pts, timebase }
    }

    /// Creates a timestamp with a validated timebase.
    pub fn try_new(pts: i64, timebase: Timebase) -> Result<Self> {
        timebase.validate()?;
        Ok(Self { pts, timebase })
    }

    /// Validates the timestamp's timebase.
    pub fn validate(self) -> Result<()> {
        self.timebase.validate()
    }

    /// Returns the timestamp in seconds.
    pub fn seconds(self) -> f64 {
        self.pts as f64 * self.timebase.seconds_per_tick()
    }

    /// Returns the timestamp in seconds after validating its timebase.
    pub fn checked_seconds(self) -> Result<f64> {
        self.validate()?;
        Ok(self.seconds())
    }

    /// Compares two timestamps by media time, even when their timebases differ.
    ///
    /// The derived [`Ord`] implementation remains structural for API
    /// compatibility. Use this method whenever chronological order matters.
    pub fn chronological_cmp(self, other: Self) -> Result<Ordering> {
        self.validate()?;
        other.validate()?;
        let left = self.pts as i128 * self.timebase.num as i128 * other.timebase.den as i128;
        let right = other.pts as i128 * other.timebase.num as i128 * self.timebase.den as i128;
        Ok(left.cmp(&right))
    }

    /// Returns whether two differently represented timestamps identify the same instant.
    pub fn same_instant(self, other: Self) -> Result<bool> {
        Ok(self.chronological_cmp(other)? == Ordering::Equal)
    }

    /// Rescales this timestamp to another timebase without losing precision.
    ///
    /// Returns an error when the destination timebase cannot represent the
    /// instant with an integral presentation timestamp.
    pub fn rescale_exact(self, timebase: Timebase) -> Result<Self> {
        self.validate()?;
        timebase.validate()?;
        let numerator =
            self.pts as i128 * self.timebase.num as i128 * timebase.den as i128;
        let denominator = self.timebase.den as i128 * timebase.num as i128;
        if numerator % denominator != 0 {
            return Err(DetectError::InvalidArgument(format!(
                "timestamp cannot be represented exactly in timebase {}/{}",
                timebase.num, timebase.den
            )));
        }
        let pts = numerator / denominator;
        let pts = i64::try_from(pts).map_err(|_| {
            DetectError::InvalidArgument("rescaled timestamp is outside the i64 range".to_string())
        })?;
        Ok(Self { pts, timebase })
    }
}

/// A half-open media-time range `[start, end)`.
///
/// The endpoints may use different valid timebases. Construction validates
/// chronological ordering without converting through floating-point seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaRange {
    /// Inclusive start timestamp.
    pub start: Timestamp,
    /// Exclusive end timestamp.
    pub end: Timestamp,
}

impl MediaRange {
    /// Creates a validated half-open media range.
    pub fn new(start: Timestamp, end: Timestamp) -> Result<Self> {
        let value = Self { start, end };
        value.validate()?;
        Ok(value)
    }

    /// Validates both endpoints and their chronological ordering.
    pub fn validate(self) -> Result<()> {
        if self.start.chronological_cmp(self.end)? == Ordering::Greater {
            return Err(DetectError::InvalidArgument(
                "media range end must not precede start".to_string(),
            ));
        }
        Ok(())
    }

    /// Returns whether this range has zero duration.
    pub fn is_empty(self) -> Result<bool> {
        Ok(self.start.chronological_cmp(self.end)? == Ordering::Equal)
    }

    /// Returns the range duration in seconds.
    pub fn duration_seconds(self) -> Result<f64> {
        self.validate()?;
        Ok(self.end.checked_seconds()? - self.start.checked_seconds()?)
    }

    /// Returns whether the timestamp lies inside this half-open range.
    pub fn contains(self, timestamp: Timestamp) -> Result<bool> {
        self.validate()?;
        timestamp.validate()?;
        Ok(self.start.chronological_cmp(timestamp)? != Ordering::Greater
            && timestamp.chronological_cmp(self.end)? == Ordering::Less)
    }

    /// Returns whether two half-open ranges overlap.
    pub fn overlaps(self, other: Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.start.chronological_cmp(other.end)? == Ordering::Less
            && other.start.chronological_cmp(self.end)? == Ordering::Less)
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
    use std::cmp::Ordering;

    use super::{
        AnalysisEvent, AudioSampleFormat, DetectError, MediaRange, PixelFormat, Timebase,
        Timestamp,
    };

    #[test]
    fn timestamp_uses_its_rational_timebase() {
        let timestamp = Timestamp::new(125, Timebase::new(1, 1_000));
        assert_eq!(timestamp.seconds(), 0.125);
    }

    #[test]
    fn validated_timebases_reject_zero_and_negative_tick_durations() {
        assert!(Timebase::try_new(1, 1_000).is_ok());
        assert!(Timebase::try_new(1, 0).is_err());
        assert!(Timebase::try_new(-1, 1_000).is_err());
    }

    #[test]
    fn timestamps_compare_chronologically_across_timebases() {
        let one_second = Timestamp::new(1, Timebase::new(1, 1));
        let nine_hundred_ms = Timestamp::new(900, Timebase::new(1, 1_000));
        let thousand_ms = Timestamp::new(1_000, Timebase::new(1, 1_000));

        assert_eq!(
            nine_hundred_ms.chronological_cmp(one_second).unwrap(),
            Ordering::Less
        );
        assert!(one_second.same_instant(thousand_ms).unwrap());
    }

    #[test]
    fn exact_rescaling_preserves_instants_and_rejects_rounding() {
        let timestamp = Timestamp::new(24, Timebase::new(1, 24));
        let milliseconds = timestamp
            .rescale_exact(Timebase::new(1, 1_000))
            .unwrap();
        assert_eq!(milliseconds.pts, 1_000);
        assert!(timestamp
            .rescale_exact(Timebase::new(1, 25))
            .is_ok());

        let one_frame = Timestamp::new(1, Timebase::new(1, 24));
        assert!(one_frame
            .rescale_exact(Timebase::new(1, 1_000))
            .is_err());
    }

    #[test]
    fn media_ranges_are_half_open_and_cross_timebase_safe() {
        let range = MediaRange::new(
            Timestamp::new(500, Timebase::new(1, 1_000)),
            Timestamp::new(2, Timebase::new(1, 1)),
        )
        .unwrap();

        assert_eq!(range.duration_seconds().unwrap(), 1.5);
        assert!(range
            .contains(Timestamp::new(1_999, Timebase::new(1, 1_000)))
            .unwrap());
        assert!(!range
            .contains(Timestamp::new(2, Timebase::new(1, 1)))
            .unwrap());
        assert!(range
            .overlaps(
                MediaRange::new(
                    Timestamp::new(1_500, Timebase::new(1, 1_000)),
                    Timestamp::new(2_500, Timebase::new(1, 1_000)),
                )
                .unwrap(),
            )
            .unwrap());
        assert!(MediaRange::new(
            Timestamp::new(3, Timebase::new(1, 1)),
            Timestamp::new(2, Timebase::new(1, 1)),
        )
        .is_err());
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

    #[test]
    fn shared_error_display_preserves_compatibility_messages() {
        assert_eq!(
            DetectError::UnsupportedPixelFormat(PixelFormat::Bgr24).to_string(),
            "unsupported pixel format: Bgr24"
        );
        assert_eq!(
            DetectError::UnsupportedAudioSampleFormat(AudioSampleFormat::I16).to_string(),
            "unsupported audio sample format: I16"
        );
        assert_eq!(
            DetectError::InvalidFrameBuffer {
                expected: 24,
                actual: 12,
            }
            .to_string(),
            "invalid frame buffer: expected at least 24 bytes, got 12"
        );
        assert_eq!(
            DetectError::InvalidDimensions {
                width: 0,
                height: 12,
            }
            .to_string(),
            "invalid dimensions: 0x12"
        );
        assert_eq!(
            DetectError::InvalidAudioFormat {
                sample_rate: 0,
                channels: 2,
            }
            .to_string(),
            "invalid audio format: sample_rate=0, channels=2"
        );
        assert_eq!(
            DetectError::Source("unavailable".into()).to_string(),
            "video source error: unavailable"
        );
        assert_eq!(
            DetectError::InvalidArgument("bad value".into()).to_string(),
            "invalid argument: bad value"
        );
        assert_eq!(
            DetectError::from(std::io::Error::other("disk")).to_string(),
            "I/O error: disk"
        );
    }
}
