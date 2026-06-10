# Text Release Scope

The first text release covers all current reusable text library crates, with
`text-analysis::TextWorkspace` as the primary package-consumer workflow and the
focused crates as lower-level escape hatches. It defines a stable contract for
public APIs, schemas, operation envelopes, and adapter behavior; NLP output
quality remains best-effort.

Default builds are deterministic, local-first, and useful without network
access, model downloads, hosted AI credentials, or native inference runtimes.
Native/model-feature builds may prefer local model execution for declared
model-backed workflows and may auto-download missing model bundles when those
side effects are explicit.

This release favors explicit contracts and reproducible fallback behavior over
claims of production-grade NLP quality. Model-backed paths may exist behind
features or caller-supplied backends, but they are not the default experience
and are not required to use the text crates.

## Major Release Contract

- Default builds are deterministic, local-first, and network-free.
- Package-surface operations do not download models, call network services,
  invoke native inference, or write persistence artifacts in default builds.
- Model-backed behavior requires explicit feature flags and workflow selection.
  `moritzbrantner-text-question-answering` with `local-onnx` uses local RoBERTa
  SQuAD2 ONNX for `qa.answer`; `moritzbrantner-text-classification` with
  `local-models` uses local DistilBERT SST-2 for classification/sentiment and
  local Xenova BART MNLI ONNX for zero-shot classification.
- Model downloads require explicit model-backed workflow selection,
  `auto_download: true`, `autoDownload: true`, or an equivalent setup command.
- Classification and question-answering model catalogs may include model
  metadata, but reference-only models must not be presented as runnable.
- Hashed embeddings and heuristic NLP are deterministic baselines, not quality
  claims.

## Text Model Release Scope

The text release surface treats user-visible model entries as either loadable or reference-only. Deterministic fallback models remain available in default builds. Native model loads require explicit feature gates and local setup:

- Tokenizers: `tokenizers,model-bundles`
- Candle token classification and embeddings: `candle,model-bundles`
- ONNX embeddings: `onnx,model-bundles`
- Candle text classification and sentiment: `text-classification/local-models`
  using `distilbert-sst2`
- ONNX zero-shot text classification: `text-classification/local-models` using
  `xenova-bart-large-mnli-onnx`
- whisper.cpp transcription: `native`
- External smoke tests: `external-tests`

Question-answering catalogs now expose
`onnx-community/roberta-base-squad2-ONNX` as runnable when built with
`local-onnx`. Native text classification was previously out of scope; this
release now requires first-party local classification models.

Release checks should include the default deterministic suite plus opt-in ignored tests only on machines with model bundles or native runtimes installed.

## Release Hardening Gates

The text package surfaces are considered releasable only when the shared audit
tests pass these gates:

- Operation IDs remain stable and match `docs/PACKAGE_SURFACE_MATRIX.md`.
- Each operation declares an explicit top-level request schema, required-field
  list, stable/additive release markers, resource-limit metadata, and a
  `workflow`, `debug`, or `support` category.
- Each operation returns the structured package-surface value shape:
  `operation`, `title`, `message`, `summary`, and `result`.
- Malformed request shapes and unknown operations return the shared typed
  `SurfaceError` envelope, and server adapters surface the same code/message in
  diagnostics.
- Default package-surface calls remain deterministic, local-first, in-memory,
  and free of downloads, native inference, network calls, and persistence
  writes unless the operation is explicitly model-backed and declares those
  side effects.

## What This Release Provides

- Shared text document, segment, span, token, sentence, and paragraph contracts.
- Unicode-aware normalization and deterministic segmentation helpers.
- Classical lexical analysis: stop words, keywords, n-grams, shingles,
  readability, stemming, sentiment, extractive summaries, user-facing lexical
  corpus assembly, reproducible corpus snapshots, TF-IDF, and BM25.
- High-level document and corpus report orchestration built from the focused
  text crates.
- Deterministic hashed embeddings, vector similarity helpers, embedding backend
  traits, and backend catalog metadata that does not load model bundles.
- Local chunking, metadata-aware retrieval, full-text search, semantic search
  over supplied embeddings, hybrid retrieval, and JSON/JSONL persistence
  helpers.
- Heuristic-first linguistic analysis with optional model-backed paths where
  feature-enabled.
- Transcript parsing, normalization, formatting, and conversion into generic
  text segments.
- Concrete task contracts for classification, extractive question answering,
  and deterministic generation/scoring fallbacks.

## What This Release Does Not Claim

- It is not a hosted LLM client layer.
- It does not download models or call network services in default builds.
- It does not require ONNX Runtime, Candle, tokenizers, whisper.cpp, or other
  native inference dependencies in default builds.
- It does not claim production-grade semantic embeddings by default; hashed
  embeddings are deterministic baselines and interoperability surfaces.
- It does not claim state-of-the-art NLP accuracy for language detection,
  morphology, syntax, entity extraction, sentiment, summarization,
  classification, question answering, or generation.
- It does not provide an aggregate "do everything NLP" crate. Task crates stay
  focused on their concrete request/response contracts.
- It does not promise open-ended generative model inference. `moritzbrantner-text-generation`
  is deterministic Markov/template-style generation from known inputs.

## Which Crate Should I Use?

Start with the smallest crate that owns the capability you need:

| Need | Start with |
| --- | --- |
| Contracts, document/segment types, spans, normalization, tokenization, sentence boundaries, or paragraph boundaries | `moritzbrantner-text-core` |
| Deterministic lexical analysis, stop words, keywords, n-grams, shingles, readability, stemming, sentiment, extractive summaries, TF-IDF, or BM25 | `moritzbrantner-text-lexical` |
| High-level document or corpus reports that orchestrate the focused text crates | `moritzbrantner-text-analysis` |
| Deterministic hashed embeddings, embedding backend traits, or backend catalog metadata | `moritzbrantner-text-embeddings` |
| Chunking, metadata-aware search, full-text/semantic/hybrid retrieval, persistence helpers, or snapshot planning | `moritzbrantner-text-retrieval` |
| Heuristic-first linguistic analysis, focused language detection, with optional model-backed paths | `moritzbrantner-text-linguistics` |
| Transcript parsing, normalization, SRT/WebVTT formatting, or transcript-to-text-segment conversion | `moritzbrantner-text-transcripts` |
| Text classification or zero-shot classification contracts and deterministic fallbacks | `moritzbrantner-text-classification` |
| Extractive question-answering contracts, deterministic/imported span handling, and optional local ONNX QA | `moritzbrantner-text-question-answering` |
| Deterministic generation contracts, Markov scoring, and template fallbacks | `moritzbrantner-text-generation` |
| Linguistic-analysis adapters for deterministic generation workflows | `moritzbrantner-text-generation-linguistics` |

Use `moritzbrantner-text-core` when you are defining data boundaries or passing text between
packages. Use `moritzbrantner-text-lexical` when you want deterministic local analysis,
lexical corpus construction, TF-IDF/BM25 scoring, or serializable lexical corpus
snapshots. Use `moritzbrantner-text-analysis` when you want a report assembled from multiple
focused crates instead of wiring them yourself.

The term "corpus" has crate-specific meanings. In `moritzbrantner-text-lexical`,
`TextCorpus` is the user-facing raw text corpus builder and `TfIdfCorpus` /
`Bm25Corpus` are scoring structures. In `moritzbrantner-text-analysis`, corpus APIs produce
multi-document reports. In `moritzbrantner-text-retrieval`, `RetrievalIndex` is a chunked,
metadata-rich search index for full-text, vector, and hybrid retrieval.
See [Text Corpus Guide](TEXT_CORPUS_GUIDE.md) for end-to-end examples across
these types.

`moritzbrantner-text-classification`, `moritzbrantner-text-question-answering`, and `moritzbrantner-text-generation` are
concrete task crates. They are intentionally not aggregate NLP mega-crates and
should not grow unrelated embedding, retrieval, summarization, or transcript
APIs.

## Stable In 0.1

The intended stable surface for `0.1` is:

- `moritzbrantner-text-core` contracts, owned/borrowed document and segment types, span types,
  normalization helpers, tokenization, sentence boundaries, paragraph
  boundaries, and conversion traits.
- `moritzbrantner-text-lexical` deterministic lexical feature APIs, `TextCorpus` builders,
  reproducible lexical corpus snapshots, TF-IDF/BM25 scoring APIs, and corpus
  statistics where outputs are derived from local text inputs.
- `moritzbrantner-text-transcripts` transcript contracts, parsers, SRT/WebVTT formatters, and
  conversion into generic text segments.
- `moritzbrantner-text-embeddings` embedding backend traits and deterministic hashed embedding
  APIs.
- `moritzbrantner-text-retrieval` chunking, retrieval request/result contracts, metadata
  filters, snapshot planning, and persistence DTOs.
- Concrete task request/response structs in `moritzbrantner-text-classification`,
  `moritzbrantner-text-question-answering`, and `moritzbrantner-text-generation`.
- First-party local classification adapters behind `text-classification/local-models`:
  Candle DistilBERT SST-2 for `classification.classify` and
  `classification.sentiment`, and ONNX pair/NLI scoring for
  `classification.zeroShot`.
- Feature policy: default builds stay local, deterministic, and free of native
  inference/runtime requirements.

Minor releases may add fields, adapters, and helper methods, but the crate
boundaries above should remain recognizable.

## Major Release Readiness

For a major release, treat the reusable library crates as the primary SemVer
contract. CLI, server, WASM, and app companions should either remain pre-1.0 or
carry a separate explicit compatibility statement before their routes,
operation grouping, app presets, and UI assumptions are considered stable.

Before promoting the text crates to a major version:

- Remove deprecated compatibility shims instead of carrying them forward.
- Keep default library constructors deterministic, local-first, and no-download.
- Require explicit constructors or request fields for model-backed behavior and
  model downloads.
- Keep transcript-specific DTOs and analyzers in `moritzbrantner-text-transcripts`;
  other crates should consume `TextSegmentContract` or `TextDocumentContract`.
- Keep package-surface example operations in-memory and artifact-free.
- Make README examples compile where possible, using `no_run` only when the
  example needs runtime setup.

## Experimental Or Best-Effort

These surfaces are useful but should be treated as experimental or
best-effort:

- Heuristic linguistic labels for POS, morphology, dependencies, coreference,
  relations, events, discourse, topics, and style.
- Sentiment, extractive summary, readability, keyword, and stemming quality
  outside the languages and cases covered by tests.
- Hashed embeddings as a proxy for semantic similarity.
- Hybrid retrieval ranking quality, score calibration, facets, and reranking.
- Optional ONNX, Candle, tokenizer, model-bundle, CUDA, whisper, and
  external-tool integrations.
- Server, CLI, and WASM adapter surfaces compared with the underlying library
  contracts.
- Imported model prediction postprocessing in task crates when callers provide
  their own model outputs.

Experimental does not mean unstable by default; it means the API exists to make
the workflow explicit, while output quality and backend coverage are expected to
improve over time.

## Release Boundary Rules

- `moritzbrantner-text-core` must not depend on specialized text crates.
- `moritzbrantner-text-retrieval` may consume `moritzbrantner-text-transcripts` only through an explicit
  optional feature.
- Model/runtime support must remain opt-in and feature-gated.
- Default builds must not add network access, model downloads, hosted API
  clients, or native inference requirements.
- New task APIs should live in focused task crates instead of creating an
  aggregate NLP facade.
