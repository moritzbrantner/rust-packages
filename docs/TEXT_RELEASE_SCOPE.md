# Text Release Scope

The first text release is a deterministic, local-first toolkit for text
contracts, lexical analysis, transcript handling, retrieval, and task-specific
request/response surfaces. It is designed to be useful without network access,
model downloads, hosted AI credentials, or native inference runtimes in default
builds.

This release favors explicit contracts and reproducible fallback behavior over
claims of production-grade NLP quality. Model-backed paths may exist behind
features or caller-supplied backends, but they are not the default experience
and are not required to use the text crates.

## What This Release Provides

- Shared text document, segment, span, token, sentence, and paragraph contracts.
- Unicode-aware normalization and deterministic segmentation helpers.
- Classical lexical analysis: stop words, keywords, n-grams, shingles,
  readability, stemming, sentiment, extractive summaries, TF-IDF, and BM25.
- High-level document and corpus report orchestration built from the focused
  text crates.
- Deterministic hashed embeddings, vector similarity helpers, and embedding
  backend traits.
- Local chunking, metadata-aware retrieval, full-text search, semantic search
  over supplied embeddings, hybrid retrieval, and JSON/JSONL persistence
  helpers.
- Heuristic-first linguistic analysis with optional model-backed paths where
  feature-enabled.
- Transcript parsing, normalization, formatting, and conversion into generic
  text segments.
- Concrete task contracts for classification, extractive question answering,
  and deterministic generation fallbacks.

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
- It does not promise open-ended generative model inference. `text-generation`
  is deterministic Markov/template-style generation from known inputs.

## Which Crate Should I Use?

Start with the smallest crate that owns the capability you need:

| Need | Start with |
| --- | --- |
| Contracts, document/segment types, spans, normalization, tokenization, sentence boundaries, or paragraph boundaries | `text-core` |
| Deterministic lexical analysis, stop words, keywords, n-grams, shingles, readability, stemming, sentiment, extractive summaries, TF-IDF, or BM25 | `text-lexical` |
| High-level document or corpus reports that orchestrate the focused text crates | `text-analysis` |
| Deterministic hashed embeddings or embedding backend traits | `text-embeddings` |
| Chunking, metadata-aware search, full-text/semantic/hybrid retrieval, or persistence helpers | `text-retrieval` |
| Heuristic-first linguistic analysis, with optional model-backed paths | `text-linguistics` |
| Transcript parsing, normalization, or formatting | `text-transcripts` |
| Text classification or zero-shot classification contracts and deterministic fallbacks | `text-classification` |
| Extractive question-answering contracts and deterministic/imported span handling | `text-question-answering` |
| Deterministic generation contracts and Markov/template fallbacks | `text-generation` |
| Linguistic-analysis adapters for deterministic generation workflows | `text-generation-linguistics` |

Use `text-core` when you are defining data boundaries or passing text between
packages. Use `text-lexical` when you want deterministic local analysis. Use
`text-analysis` when you want a report assembled from multiple focused crates
instead of wiring them yourself.

`text-classification`, `text-question-answering`, and `text-generation` are
concrete task crates. They are intentionally not aggregate NLP mega-crates and
should not grow unrelated embedding, retrieval, summarization, or transcript
APIs.

## Stable In 0.1

The intended stable surface for `0.1` is:

- `text-core` contracts, owned/borrowed document and segment types, span types,
  normalization helpers, tokenization, sentence boundaries, paragraph
  boundaries, and conversion traits.
- `text-lexical` deterministic lexical feature APIs and corpus statistics where
  outputs are derived from local text inputs.
- `text-transcripts` transcript contracts, parsers, formatters, and conversion
  into generic text segments.
- `text-embeddings` embedding backend traits and deterministic hashed embedding
  APIs.
- `text-retrieval` chunking, retrieval request/result contracts, metadata
  filters, and persistence DTOs.
- Concrete task request/response structs in `text-classification`,
  `text-question-answering`, and `text-generation`.
- Feature policy: default builds stay local, deterministic, and free of native
  inference/runtime requirements.

Minor releases may add fields, adapters, and helper methods, but the crate
boundaries above should remain recognizable.

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

- `text-core` must not depend on specialized text crates.
- `text-retrieval` may consume `text-transcripts` only through an explicit
  optional feature.
- Model/runtime support must remain opt-in and feature-gated.
- Default builds must not add network access, model downloads, hosted API
  clients, or native inference requirements.
- New task APIs should live in focused task crates instead of creating an
  aggregate NLP facade.
