# text-question-answering

Concrete extractive question-answering contracts for `moritzbrantner-video-analysis`.

Model download and bundle handling belong to `moritzbrantner-model-runtime`; this crate owns
the question/context request shape, answer predictions, imported span
postprocessing, and fallback behavior.

## Stable contract

The stable surface is extractive QA request/response DTOs, imported span
postprocessing, model catalog metadata, and runtime broker traits supplied by
callers.

## Quality and limits

Default package operations postprocess supplied spans and do not run native QA
models. Catalog entries that are not `loadable` are reference metadata, not
runnable native models.

## Package surface

- Primary workflow: `qa.answer` postprocesses imported extractive QA span
  predictions for a supplied question and context.
- Workflow operations: `qa.answer`, `qa.answerWithRetrieval`, and
  `qa.answerBatch`.
- Debug operations: `qa.models` inspects the model catalog, and `describe`
  inspects package metadata and operation support.
- Runtime support: pure Rust default behavior is available through library, CLI,
  server, and WASM wrappers.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific answer, citation, batch-result, and retrieval fields.
- The package surface does not download model bundles or execute native QA
  runtimes.
