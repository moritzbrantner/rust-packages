# ADR 0002: Text Release Classical And Local Models

## Status

Accepted

## Context

The text crates need a first release boundary that package consumers can rely
on without mistaking stable APIs for production-grade NLP quality. The workspace
already contains deterministic classical text crates and opt-in local model
runtime infrastructure. Classification previously exposed imported-prediction
and lexical fallback contracts, but did not provide first-party local
classification runners.

## Decision

The first text release includes all current reusable text library crates.
`text-analysis::TextWorkspace` is the primary package-consumer workflow, while
focused crates remain lower-level ownership boundaries.

The release promise is a stable contract: public APIs, schemas, operation
envelopes, adapter behavior, and compatibility rules are stable/additive.
Heuristic and model outputs are best-effort results.

Default builds remain deterministic, model-free, network-free, and
no-download. Model-capable crates may expose explicit native/model-feature
workflows. Those workflows may prefer local models and may auto-download missing
model bundles only when the side effect is declared by feature flags and local
model options.

For classification:

- `classification.classify` uses local Candle DistilBERT SST-2 when
  `text-classification/local-models` and local model options are selected.
- `classification.sentiment` uses local Candle DistilBERT SST-2 as the first
  releasable sentiment model.
- `classification.zeroShot` uses local ONNX pair/NLI classification with
  `xenova-bart-large-mnli-onnx`.
- Caller-supplied backends and imported predictions remain supported.
- Zero-shot uses a separate pair-classification trait instead of overloading
  ordinary sequence classification.

## Consequences

Package consumers can depend on stable request/response shapes even when output
quality improves over time. Default CI and default library constructors stay
fast and deterministic. Native/model-feature workflows have clearer side
effects, but they require more explicit documentation, ignored external smoke
tests, and model bundle setup guidance.

`TextWorkspace` orchestrates classification through `text-classification`; it
does not own tokenizer, Candle, ONNX, or model bundle internals.
