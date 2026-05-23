#![doc = include_str!("../README.md")]

pub mod surface;
use video_analysis_core::{DetectError, Result};

fn invalid_argument(message: impl Into<String>) -> DetectError {
    DetectError::InvalidArgument(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Data type for sample rate.
pub struct SampleRate(pub u32);

impl SampleRate {
    /// Creates a new value.
    pub fn new(value: u32) -> Result<Self> {
        if value == 0 {
            return Err(DetectError::InvalidAudioFormat {
                sample_rate: value,
                channels: 1,
            });
        }
        Ok(Self(value))
    }

    /// Returns get.
    pub fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for resample ratio.
pub struct ResampleRatio {
    /// The input value.
    pub input: SampleRate,
    /// The output value.
    pub output: SampleRate,
}

impl ResampleRatio {
    /// Creates a new value.
    pub fn new(input: SampleRate, output: SampleRate) -> Result<Self> {
        let ratio = Self { input, output };
        ratio.validate()?;
        Ok(ratio)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        SampleRate::new(self.input.get())?;
        SampleRate::new(self.output.get())?;
        Ok(())
    }

    /// Borrows this value as a f64.
    pub fn as_f64(self) -> f64 {
        self.output.get() as f64 / self.input.get() as f64
    }

    /// Returns source position for output.
    pub fn source_position_for_output(self, output_index: usize) -> f64 {
        output_index as f64 * self.input.get() as f64 / self.output.get() as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing interpolation mode.
pub enum InterpolationMode {
    /// The nearest variant.
    Nearest,
    /// The linear variant.
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for resample spec.
pub struct ResampleSpec {
    /// The input value.
    pub input: SampleRate,
    /// The output value.
    pub output: SampleRate,
    /// The interpolation value.
    pub interpolation: InterpolationMode,
}

impl ResampleSpec {
    /// Creates a new value.
    pub fn new(
        input: SampleRate,
        output: SampleRate,
        interpolation: InterpolationMode,
    ) -> Result<Self> {
        let spec = Self {
            input,
            output,
            interpolation,
        };
        spec.validate()?;
        Ok(spec)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        ResampleRatio::new(self.input, self.output)?;
        Ok(())
    }

    /// Returns ratio.
    pub fn ratio(self) -> ResampleRatio {
        ResampleRatio {
            input: self.input,
            output: self.output,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing window function.
pub enum WindowFunction {
    /// The rectangular variant.
    Rectangular,
    /// The hann variant.
    Hann,
    /// The hamming variant.
    Hamming,
    /// The blackman variant.
    Blackman,
}

impl WindowFunction {
    /// Returns coefficient.
    pub fn coefficient(self, index: usize, len: usize) -> f32 {
        if len <= 1 {
            return 1.0;
        }
        let phase = 2.0 * std::f32::consts::PI * index as f32 / (len - 1) as f32;
        match self {
            Self::Rectangular => 1.0,
            Self::Hann => 0.5 - 0.5 * phase.cos(),
            Self::Hamming => 0.54 - 0.46 * phase.cos(),
            Self::Blackman => 0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos(),
        }
    }

    /// Returns weights.
    pub fn weights(self, len: usize) -> Vec<f32> {
        (0..len).map(|index| self.coefficient(index, len)).collect()
    }

    /// Returns apply.
    pub fn apply(self, samples: &[f32]) -> Vec<f32> {
        samples
            .iter()
            .enumerate()
            .map(|(index, sample)| sample * self.coefficient(index, samples.len()))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for window spec.
pub struct WindowSpec {
    /// The function value.
    pub function: WindowFunction,
    /// The len value.
    pub len: usize,
}

impl WindowSpec {
    /// Creates a new value.
    pub fn new(function: WindowFunction, len: usize) -> Result<Self> {
        let spec = Self { function, len };
        spec.validate()?;
        Ok(spec)
    }

    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if self.len == 0 {
            return Err(invalid_argument("window length must be greater than zero"));
        }
        Ok(())
    }

    /// Returns weights.
    pub fn weights(self) -> Vec<f32> {
        self.function.weights(self.len)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Data type for frame stride.
pub struct FrameStride {
    /// The frame size value.
    pub frame_size: usize,
    /// The hop size value.
    pub hop_size: usize,
}

impl FrameStride {
    /// Creates a new value.
    pub fn new(frame_size: usize, hop_size: usize) -> Result<Self> {
        if frame_size == 0 || hop_size == 0 {
            return Err(invalid_argument(
                "frame_size and hop_size must be greater than zero",
            ));
        }
        Ok(Self {
            frame_size,
            hop_size,
        })
    }

    /// Returns frame count.
    pub fn frame_count(self, samples_len: usize) -> usize {
        if samples_len < self.frame_size {
            return 0;
        }
        1 + (samples_len - self.frame_size) / self.hop_size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Variants describing biquad design.
pub enum BiquadDesign {
    /// The low pass variant.
    LowPass,
    /// The high pass variant.
    HighPass,
    /// The band pass variant.
    BandPass,
    /// The notch variant.
    Notch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data type for biquad coefficients.
pub struct BiquadCoefficients {
    /// The b0 value.
    pub b0: f32,
    /// The b1 value.
    pub b1: f32,
    /// The b2 value.
    pub b2: f32,
    /// The a1 value.
    pub a1: f32,
    /// The a2 value.
    pub a2: f32,
}

impl BiquadCoefficients {
    /// Validates this value.
    pub fn validate(self) -> Result<()> {
        if [self.b0, self.b1, self.b2, self.a1, self.a2]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(invalid_argument("biquad coefficients must be finite"));
        }
        Ok(())
    }
}

impl BiquadDesign {
    /// Returns design.
    pub fn design(
        self,
        sample_rate: SampleRate,
        cutoff_hz: f32,
        q: f32,
    ) -> Result<BiquadCoefficients> {
        if !cutoff_hz.is_finite() || cutoff_hz <= 0.0 || !q.is_finite() || q <= 0.0 {
            return Err(invalid_argument(
                "biquad cutoff must be finite and positive, and q must be finite and positive",
            ));
        }
        let nyquist = sample_rate.get() as f32 / 2.0;
        if cutoff_hz >= nyquist {
            return Err(invalid_argument("biquad cutoff must be below Nyquist"));
        }
        let omega = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate.get() as f32;
        let cos = omega.cos();
        let sin = omega.sin();
        let alpha = sin / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match self {
            Self::LowPass => (
                (1.0 - cos) / 2.0,
                1.0 - cos,
                (1.0 - cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            Self::HighPass => (
                (1.0 + cos) / 2.0,
                -(1.0 + cos),
                (1.0 + cos) / 2.0,
                1.0 + alpha,
                -2.0 * cos,
                1.0 - alpha,
            ),
            Self::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            Self::Notch => (1.0, -2.0 * cos, 1.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
        };
        BiquadCoefficients {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
        .validate()?;
        Ok(BiquadCoefficients {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Data type for fir kernel1d.
pub struct FirKernel1d {
    values: Vec<f32>,
}

impl FirKernel1d {
    /// Creates a new value.
    pub fn new(values: impl Into<Vec<f32>>) -> Result<Self> {
        let kernel = Self {
            values: values.into(),
        };
        kernel.validate()?;
        Ok(kernel)
    }

    /// Returns values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Validates this value.
    pub fn validate(&self) -> Result<()> {
        if self.values.is_empty() {
            return Err(invalid_argument("FIR kernel must not be empty"));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(invalid_argument("FIR kernel values must be finite"));
        }
        Ok(())
    }
}

/// Returns interpolate at.
pub fn interpolate_at(samples: &[f32], position: f64, mode: InterpolationMode) -> Result<f32> {
    if samples.is_empty() {
        return Err(invalid_argument("samples must not be empty"));
    }
    if !position.is_finite() || position < 0.0 {
        return Err(invalid_argument(
            "interpolation position must be finite and non-negative",
        ));
    }
    let max_index = samples.len().saturating_sub(1) as f64;
    let clamped = position.min(max_index);
    Ok(match mode {
        InterpolationMode::Nearest => samples[clamped.round() as usize],
        InterpolationMode::Linear => {
            let left = clamped.floor() as usize;
            let right = clamped.ceil() as usize;
            if left == right {
                samples[left]
            } else {
                let fraction = (clamped - left as f64) as f32;
                samples[left] * (1.0 - fraction) + samples[right] * fraction
            }
        }
    })
}

/// Returns resample indices.
pub fn resample_indices(spec: ResampleSpec, output_len: usize) -> Result<Vec<f64>> {
    spec.validate()?;
    Ok((0..output_len)
        .map(|index| spec.ratio().source_position_for_output(index))
        .collect())
}

/// Returns FFT bin frequency.
pub fn fft_bin_frequency(bin: usize, fft_size: usize, sample_rate: SampleRate) -> Result<f32> {
    if fft_size == 0 {
        return Err(invalid_argument("fft_size must be greater than zero"));
    }
    if bin > fft_size / 2 {
        return Err(invalid_argument("fft bin must not exceed the Nyquist bin"));
    }
    Ok(bin as f32 * sample_rate.get() as f32 / fft_size as f32)
}

/// Returns frequency to FFT bin.
pub fn frequency_to_fft_bin(
    frequency_hz: f32,
    fft_size: usize,
    sample_rate: SampleRate,
) -> Result<usize> {
    if !frequency_hz.is_finite() || frequency_hz < 0.0 {
        return Err(invalid_argument(
            "frequency must be finite and non-negative",
        ));
    }
    if fft_size == 0 {
        return Err(invalid_argument("fft_size must be greater than zero"));
    }
    let nyquist = sample_rate.get() as f32 / 2.0;
    if frequency_hz > nyquist {
        return Err(invalid_argument("frequency must not exceed Nyquist"));
    }
    Ok((frequency_hz * fft_size as f32 / sample_rate.get() as f32).round() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_lengths_and_symmetry_match_expectations() {
        let weights = WindowFunction::Hann.weights(4);
        assert_eq!(weights.len(), 4);
        assert!((weights[0] - weights[3]).abs() < 1.0e-6);
    }

    #[test]
    fn interpolation_handles_edge_cases() {
        assert_eq!(
            interpolate_at(&[0.0, 10.0], 0.5, InterpolationMode::Linear).unwrap(),
            5.0
        );
        assert_eq!(
            interpolate_at(&[0.0, 10.0], 2.0, InterpolationMode::Nearest).unwrap(),
            10.0
        );
    }

    #[test]
    fn resample_and_filter_descriptors_validate() {
        let ratio = ResampleRatio::new(
            SampleRate::new(48_000).unwrap(),
            SampleRate::new(16_000).unwrap(),
        )
        .unwrap();
        assert!(ratio.as_f64() < 1.0);
        assert!(BiquadDesign::LowPass
            .design(SampleRate::new(48_000).unwrap(), 1_000.0, 0.707)
            .is_ok());
        assert!(FirKernel1d::new([0.25, 0.5, 0.25]).is_ok());
    }

    #[test]
    fn fft_frequency_helpers_round_trip() {
        let sample_rate = SampleRate::new(8_000).unwrap();
        let bin = frequency_to_fft_bin(1_000.0, 8, sample_rate).unwrap();
        assert_eq!(bin, 1);
        assert_eq!(fft_bin_frequency(bin, 8, sample_rate).unwrap(), 1_000.0);
    }
}
