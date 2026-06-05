# math-signal-core

Shared signal-domain math for windows, frame strides, resampling, and biquad
design.

## Highlights

- Checked sample-rate and resampling descriptors
- Shared window functions and frame/hop sizing
- Interpolation helpers for signal-domain consumers
- Reusable FIR and biquad coefficient contracts
- Signal level summaries, centered FIR application, and peak normalization

## Example

```rust,no_run
use math_signal_core::{BiquadDesign, FrameStride, SampleRate, WindowFunction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = FrameStride::new(1024, 256)?;
    let weights = WindowFunction::Hann.weights(4);
    let coeffs = BiquadDesign::LowPass.design(SampleRate::new(48_000)?, 1_000.0, 0.707)?;
    assert_eq!(spec.frame_count(2_048), 5);
    assert!(weights[1] > 0.5);
    coeffs.validate()?;
    Ok(())
}
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-processing`
- `audio-analysis-fourier`
