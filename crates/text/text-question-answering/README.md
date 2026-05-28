# text-question-answering

Concrete extractive question-answering contracts for `video-analysis`.

Model download and bundle handling belong to `model-runtime`; this crate owns
the question/context request shape, answer predictions, imported span
postprocessing, and fallback behavior.

## Package surface

- Primary workflow: `qa.answer` postprocesses imported extractive QA span
  predictions for a supplied question and context.
- Workflow operations: `qa.answer`.
- Debug operations: `qa.models` inspects the model catalog, and `describe`
  inspects package metadata and operation support.
- Runtime support: pure Rust default behavior is available through library, CLI,
  server, and WASM wrappers.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific answer fields.
- The package surface does not download model bundles or execute native QA
  runtimes.
