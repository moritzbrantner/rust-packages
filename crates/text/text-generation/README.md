# text-generation

Deterministic Markov-chain text prediction and template synthesis for
`moritzbrantner-video-analysis`. This crate does not include hosted LLM clients or native
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

## Package surface

- Primary workflow: `generation.markovGenerate` trains a transient Markov chain
  and deterministically generates text.
- Workflow operations: `generation.markovPredict`,
  `generation.markovGenerate`, `generation.perplexity`, and
  `generation.synthesizeTerms`.
- Debug operations: `describe` inspects package metadata and operation support.
- Runtime support: pure Rust, available through library, CLI, server, and WASM
  wrappers.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific fields such as `predictions`, `generation`, `perplexity`,
  `value`, or `trace`.
- This crate does not include hosted LLM clients or native open-ended
  generative model inference.

## Related crates

- `text-core`
- `text-generation-linguistics`
