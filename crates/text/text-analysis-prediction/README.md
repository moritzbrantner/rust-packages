# text-analysis-prediction

Deterministic Markov-chain text prediction for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use text_analysis_linguistics::{analyze_text, LinguisticAnalysisOptions};
use text_analysis_prediction::{MarkovChain, MarkovInputMode};

let analysis = analyze_text(
    "Scene transitions follow strong visual changes.",
    &LinguisticAnalysisOptions::default(),
) .unwrap();
let mut model = MarkovChain::new(2).unwrap();
model.train_analysis(&analysis, MarkovInputMode::Lemma);

let _next = model.predict_next(["scene", "transitions"], 3).unwrap();
```

## Related crates

- `text-analysis-core`
- `text-analysis-synthesis`
