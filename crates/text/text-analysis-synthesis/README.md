# text-analysis-synthesis

Deterministic text synthesis from terms and events for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_synthesis::{generate_document, WeightedTerm};

let terms = vec![WeightedTerm::new("analysis", 0.9), WeightedTerm::new("scene", 0.7)];
let document = generate_document(&terms)?;

let _ = document;
```

## Related crates

- `data-inversion-core`
- `text-analysis-core`
