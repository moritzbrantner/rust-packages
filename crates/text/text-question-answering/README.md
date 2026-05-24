# text-question-answering

Concrete extractive question-answering contracts for `video-analysis`.

Model download and bundle handling belong to `model-runtime`; this crate owns
the question/context request shape, answer predictions, imported span
postprocessing, and fallback behavior.

