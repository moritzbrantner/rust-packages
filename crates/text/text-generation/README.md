# text-generation

Deterministic Markov-chain text prediction and template synthesis for
`video-analysis`. This crate does not include hosted LLM clients or native
open-ended generative model inference.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use text_linguistics::{analyze_text, LinguisticAnalysisOptions};
use text_generation::{MarkovChain, MarkovInputMode};

let analysis = analyze_text(
    "Scene transitions follow strong visual changes.",
    &LinguisticAnalysisOptions::default(),
) .unwrap();
let mut model = MarkovChain::new(2).unwrap();
model.train_analysis(&analysis, MarkovInputMode::Lemma);

let _next = model.predict_next(["scene", "transitions"], 3).unwrap();
```

## Related crates

- `text-core`
- `text-generation`
