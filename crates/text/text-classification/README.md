# text-classification

Concrete text classification, sentiment, and zero-shot classification contracts
for `video-analysis`.

Model download and bundle handling belong to `model-runtime`; this crate owns
the text-facing request/response types, imported prediction handling, and
deterministic fallback behavior.

