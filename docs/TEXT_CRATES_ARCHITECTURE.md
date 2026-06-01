# Text Crates Architecture

The text crates are local-first. Default builds must not require network access,
model downloads, hosted AI credentials, or native inference runtimes. Model
execution is opt-in through feature flags and explicit runtime configuration.

## Model Loading Contract

Text package model catalogs distinguish deterministic, loadable, and reference-only entries:

- `supported: true, loadable: true` means the default or selected runtime can run without extra model setup.
- `supported: true, loadable: false` means the crate has an implemented opt-in native path, but the local bundle must be materialized first.
- `supported: false, loadable: false` means the entry is reference metadata only. Classification sequence models and extractive QA models remain in this state until native runners are implemented.

`moritzbrantner-text-model-runtime` owns the shared conformance report types: `TextModelLoadReport`, `TextModelRunReport`, `TextModelCapability`, `validate_text_model_bundle`, and `validate_tokenizer_bundle`.

Default builds remain deterministic and network-free. Native tokenizers, Candle, ONNX, model bundles, and whisper.cpp paths are opt-in through feature gates such as `tokenizers`, `candle`, `onnx`, `model-bundles`, `native`, and `external-tests`.

Use the existing bundle sync flow before running native smoke tests:

```bash
scripts/sync_model_bundles.sh
cargo test -p moritzbrantner-text-model-runtime --features external-tests -- --ignored
cargo test -p moritzbrantner-text-linguistics --features external-tests -- --ignored
cargo test -p moritzbrantner-text-embeddings --features external-tests -- --ignored
cargo test -p moritzbrantner-text-transcripts --features native,external-tests -- --ignored
```

## Benchmarks

Native Criterion benches cover segmentation, linguistics, embeddings, lexical corpus search, retrieval indexing, and text analysis reports:

```bash
bun run text-native:bench
```

Browser WASM benchmarks run in Playwright and measure the current browser and machine:

```bash
bun run text-wasm:bench:all
bun run text:bench
```

Benchmark results are not portable performance claims; they depend on CPU, browser, build profile, and current package inputs.

## Responsibilities

| Crate | Owns | Must not own |
| --- | --- | --- |
| `text-core` | Text documents, Unicode-safe spans, tokenization, sentence and paragraph boundaries, annotation graph primitives. | Model downloads, native inference, corpus search, transcript formats, transport concerns. |
| `text-lexical` | Deterministic lexical features, stop words, keywords, `TextCorpus` raw lexical corpus assembly, reproducible lexical snapshots, TF-IDF, BM25, rule entities, extractive summaries, lexical sentiment. | ASR, transcript-specific source adapters, chunked retrieval storage, native model execution. |
| `text-model-runtime` | Shared tokenizer bundles, tokenized model inputs, runtime backend traits, and optional native model facade types. | High-level NLP schemas, retrieval indexes, transcript parsing, text pipeline orchestration. |
| `text-linguistics` | Heuristic-first linguistic pipeline: language, lemmas, POS, morphology, syntax, entities, coreference, events, discourse, topics, style; optional model-backed paths. | Generic task schemas, vector retrieval storage, transcript file formats. |
| `text-embeddings` | Embedding backends, pooling, hashed fallback vectors, semantic search indexes. | General text classification, transcript parsing, linguistic annotations. |
| `text-retrieval` | Chunking, metadata filters, metadata-rich `RetrievalIndex` workflows, BM25/vector/hybrid retrieval, persistence helpers. | Embedding model internals, ASR, linguistic parsing. |
| `text-transcripts` | Transcript formats, transcript-specific analyzers, and optional ASR command/native adapters. | Generic lexical features, retrieval ranking. |
| `text-classification` | Text classification, zero-shot classification, sentiment request/response contracts, imported-prediction handling, deterministic fallbacks, runtime broker APIs. | Tokenizer implementation details, direct download policy, retrieval indexes, transcript parsing. |
| `text-question-answering` | Extractive QA request/response contracts and imported span postprocessing. | Text classification, retrieval indexes, transcript parsing. |
| `text-generation` | Deterministic Markov prediction and template/text synthesis from known signals. | Hosted LLM clients or claims of open-ended generative model inference. |
| `text-generation-linguistics` | Adapters from linguistic analyses into deterministic generation prompts, synthesis, and Markov training. | Core Markov/synthesis ownership, hosted LLM clients, native model inference. |

## Classical And Model-Backed Coverage

| Capability | Classical path | Model-backed path |
| --- | --- | --- |
| Tokenization and spans | `text-core` Unicode/token rules | `text-model-runtime` tokenizer bundles when feature-enabled |
| Lexical search | `text-lexical` TF-IDF/BM25 | Hybrid with embeddings in `text-retrieval` |
| Embeddings | `HashedTextEmbedder` | Optional ONNX/Candle embedders in `text-embeddings` |
| Linguistic analysis | Heuristic pipeline in `text-linguistics` | Optional local sequence labeler for NER |
| Transcription | Transcript parsers | Whisper CLI/native whisper.cpp adapters |
| Classification/sentiment | `text-classification` lexical/imported fallbacks | Runtime-broker traits supplied by callers |
| Question answering | `text-question-answering` imported span postprocessing | Runtime-broker traits supplied by callers |
| Reranking | `text-retrieval` ranking APIs | Runtime-broker traits supplied by callers |
| Generation | `text-generation` Markov/template synthesis; `text-generation-linguistics` analysis adapters | No hosted or native LLM path today |

## Dependency Direction

`text-core` stays dependency-light and model-free. Higher-level crates may depend
on `text-core`, but `text-core` must not depend on NLP, model, transcript, or
retrieval crates.

`text-core` also owns the generic text contract interfaces:
`TextDocumentContract`, `TextSegmentContract`, `IntoTextDocumentContract`, and
`AsTextSegmentContract`. These are the stable DTOs and conversion traits that
non-text packages should consume when they need to hand text into the text
stack.

`text-lexical` uses those generic contracts for local lexical corpus assembly.
`TextCorpus` owns raw document text, language, and metadata and can derive
`TfIdfCorpus` or `Bm25Corpus` scoring structures without changing their existing
APIs. `TextCorpusSnapshot` serializes deterministic TF-IDF term state for
reproducible local round trips.

`text-retrieval` remains the owner for chunked retrieval workflows.
`RetrievalIndex` is the metadata-rich search index for full-text, vector, and
hybrid retrieval; it should not be treated as the same abstraction as a
`text-lexical` corpus.

`text-transcripts` owns transcript extensions such as
`TranscriptSegmentContract`, `TranscriptWordContract`, and
`TranscriptionContract`. A transcript is treated as timed text with optional
speaker, confidence, and word-level metadata, and it must convert into the
generic `text-core` segment contract for lexical, linguistic, retrieval, and
pipeline consumers.

Audio ASR/model crates should return `TranscriptionContract` rather than
defining their own transcript DTO. Compatibility structs may remain temporarily
when deprecated and converted into the transcript contract at the boundary.
Audio callers should use `TranscriptionContract::from_segments` for imported
ASR segments and `text_or_joined` when they need display text, so transcript
normalization, language propagation, confidence clamping, and validation stay
centralized in `text-transcripts`.

Speaker diarization and other audio post-processing crates may enrich
`TranscriptionContract` values with speaker metadata at the audio boundary, but
they must not define transcript DTOs of their own. Linguistic analysis accepts
the transcript contract directly behind the `transcripts` feature and converts
through the existing transcript analysis path.

`text-model-runtime` is the only text crate that should define reusable tokenizer
runtime inputs and backend traits. Crates may implement or consume those traits,
but should not duplicate `TokenizedText`, `TokenizerBundle`, or runtime backend
enums.

Concrete capability crates broker task-level behavior. They may accept
caller-supplied runtime backends, imported predictions, or explicit fallback
policies. They should not silently download models or make native inference
mandatory; reusable model acquisition belongs in `model-runtime`.

## Feature Policy

Default features are deterministic and network-free. Optional runtime features
may enable tokenizers, ONNX Runtime, or Candle, but callers must still select or
provide the runtime explicitly. External tests that require real tools, models,
or network access remain behind `external-tests`.

Text Candle server binaries default to CPU. Native CUDA execution is opt-in with
the server `cuda` feature and startup flags:

```bash
cargo run -p moritzbrantner-text-analysis-server -- --addr 127.0.0.1:3000
cargo run -p moritzbrantner-text-analysis-server --features cuda -- --cuda --cuda-device-index 0
```

CUDA requires a CUDA-capable host and Candle CUDA build prerequisites. WASM
Candle surfaces remain CPU-only and must not enable the native `cuda` feature.
