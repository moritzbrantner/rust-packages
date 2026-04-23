# data-inversion-core

Shared fidelity and inversion trace metadata for generated analysis outputs.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use data_inversion_core::{InformationFidelity, InversionTrace};

let trace = InversionTrace::new(InformationFidelity::Approximate)
    .with_assumption("interpolated from adjacent frames");

let _ = trace;
```

## Related crates

- `audio-analysis-synthesis`
- `image-analysis-synthesis`
- `text-analysis-synthesis`
