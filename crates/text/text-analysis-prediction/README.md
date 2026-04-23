# text-analysis-prediction

Deterministic Markov-chain text prediction for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use text_analysis_prediction::MarkovModel;

let mut model = MarkovModel::new(2)?;
model.train("scene transitions follow strong visual changes");

let _next = model.predict_next("scene transitions", 3)?;
```

## Related crates

- `text-analysis-core`
- `text-analysis-synthesis`
