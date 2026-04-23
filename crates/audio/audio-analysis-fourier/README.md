# audio-analysis-fourier

FFT, STFT, and spectral audio analysis for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_fourier::{DominantFrequencyAnalyzer, SpectrogramOptions};

let analyzer = DominantFrequencyAnalyzer::default();
let options = SpectrogramOptions {
    fft_size: 2_048,
    hop_size: 512,
    ..SpectrogramOptions::default()
};

let _ = analyzer;
let _ = options;
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-pitch`
- `audio-analysis-rhythm`
