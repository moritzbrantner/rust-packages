# audio-analysis-recognition

Deterministic audio embeddings and similarity search for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_recognition::{ReferenceLibrary, SpectralEmbeddingConfig};

let config = SpectralEmbeddingConfig::default();
let mut library = ReferenceLibrary::new(config.embedding_dimensions())?;

library.add_reference("intro", "opening music", vec![0.0; config.embedding_dimensions()])?;
let _ = library;
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-fourier`
- `vector-analysis-core`
