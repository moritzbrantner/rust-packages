# text-generation

Deterministic Markov-chain text prediction and template synthesis for
`video-analysis`. This crate does not include hosted LLM clients or native
open-ended generative model inference.

## Feature flags

- No optional feature flags today.

## Example

```rust,no_run
use text_generation::MarkovChain;

let mut model = MarkovChain::new(2).unwrap();
model.train_text("Scene transitions follow strong visual changes.");

let _next = model.predict_next(["scene", "transitions"], 3).unwrap();
```

## Related crates

- `text-core`
- `text-generation-linguistics`
