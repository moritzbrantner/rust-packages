# Text Crates Architecture

The text crates are local-first. Default builds must not require network access,
model downloads, hosted AI credentials, or native inference runtimes unless an
operation is explicitly marked as a native server model workflow. Model
execution is opt-in through feature flags and operation/runtime configuration.

## Model Loading Contract

Text package model catalogs distinguish deterministic, loadable, and reference-only entries:

- `supported: true, loadable: true` means the default or selected runtime can run without extra model setup.
- `supported: true, loadable: false` means the crate has an implemented opt-in native path, but the local bundle must be materialized first.
- `supported: false, loadable: false` means the entry is reference metadata
  only. Extractive QA is runnable for the RoBERTa SQuAD2 ONNX preset when
  `local-onnx` is enabled. Text classification is runnable for DistilBERT
  SST-2 and Xenova BART MNLI when `text-classification/local-models` is enabled.

`moritzbrantner-text-model-runtime` owns the shared conformance report types: `TextModelLoadReport`, `TextModelRunReport`, `TextModelCapability`, `validate_text_model_bundle`, and `validate_tokenizer_bundle`.

Default builds remain deterministic and network-free. Native tokenizers,
Candle, ONNX, model bundles, and whisper.cpp paths are opt-in through feature
gates such as `tokenizers`, `candle`, `onnx`, `model-bundles`, `native`, and
`external-tests`. Feature gates make native/model paths available; callers must
still explicitly select model-backed behavior. Downloads never happen through
generic validation helpers. The explicit exceptions are model-backed runtime
operations such as `runtime.onnxQaProbe`, `runtime.downloadBundle`, `qa.answer`
with `local-onnx`, and classification package-surface operations built with
`local-models`; those operations resolve or download through
`moritzbrantner-model-runtime` when their local model options allow it. WASM
reports native-only paths as unsupported/server-only.

Use the existing bundle sync flow before running native smoke tests:

```bash
scripts/sync_model_bundles.sh
cargo test -p moritzbrantner-text-model-runtime --features external-tests -- --ignored
cargo test -p moritzbrantner-text-linguistics --features external-tests -- --ignored
cargo test -p moritzbrantner-text-embeddings --features external-tests -- --ignored
cargo test -p moritzbrantner-text-classification --features external-tests -- --ignored
cargo test -p moritzbrantner-text-transcripts --features native,external-tests -- --ignored
```

## Model-Capable And Model-Free Crates

Model-capable text crates expose domains where both deterministic and
local-model-backed execution are natural:

- `text-analysis`
- `text-classification`
- `text-embeddings`
- `text-linguistics`
- `text-model-runtime`
- `text-question-answering`
- `text-retrieval` for reranking
- `text-transcripts`

Model-free text crates for this release are deterministic by ownership:

- `text-core`
- `text-lexical`
- `text-index`
- `text-generation`
- `text-generation-linguistics`

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
| `text-model-runtime` | Shared tokenizer bundles, tokenized model inputs, runtime backend traits, optional native model facade types, Candle sequence classification, ONNX pair classification, and the server-only ONNX QA probe. | High-level NLP schemas, retrieval indexes, transcript parsing, text pipeline orchestration. |
| `text-linguistics` | Heuristic-first linguistic pipeline: language, lemmas, POS, morphology, syntax, entities, coreference, events, discourse, topics, style; optional model-backed paths. | Generic task schemas, vector retrieval storage, transcript file formats. |
| `text-embeddings` | Embedding backends, pooling, hashed fallback vectors, semantic search indexes. | General text classification, transcript parsing, linguistic annotations. |
| `text-index` | Generic contract ingestion into index documents, durable text indexes, deterministic chunking, in-memory and SQLite storage, lexical/semantic/hybrid search, semantic facets, analysis attachments, index inspection, and snapshot planning. | File extraction, hosted search services, external vector databases, graph databases, model-backed default embeddings, NLP facet derivation. |
| `text-retrieval` | Soft-legacy compatibility `RetrievalIndex`, search document adapters, reranking, and import paths from existing persisted retrieval snapshots into `text-index`. | New durable index ownership, canonical chunking for new workflows, file extraction, embedding model internals, ASR, linguistic parsing. |
| `text-transcripts` | Transcript formats, transcript-specific analyzers, and optional ASR command/native adapters. | Generic lexical features, retrieval ranking. |
| `text-classification` | Text classification, zero-shot classification, sentiment request/response contracts, imported-prediction handling, deterministic fallbacks, runtime broker APIs, and classification model policy. | Tokenizer implementation details, reusable model runtime internals, retrieval indexes, transcript parsing. |
| `text-question-answering` | Extractive QA request/response contracts, primary text-index path for cited document QA, soft-legacy compatibility retrieval-backed QA, imported span postprocessing, fallback policy, and optional local ONNX QA execution. | Text classification, tokenizer internals, transcript parsing, durable index sessions. |
| `text-generation` | Deterministic Markov prediction and template/text synthesis from known signals. | Hosted LLM clients or claims of open-ended generative model inference. |
| `text-generation-linguistics` | Adapters from linguistic analyses into deterministic generation prompts, synthesis, and Markov training. | Core Markov/synthesis ownership, hosted LLM clients, native model inference. |

## Classical And Model-Backed Coverage

| Capability | Classical path | Model-backed path |
| --- | --- | --- |
| Tokenization and spans | `text-core` Unicode/token rules | `text-model-runtime` tokenizer bundles when feature-enabled |
| Lexical search | `text-lexical` TF-IDF/BM25 | Hybrid with embeddings in `text-index`; legacy hybrid search remains in `text-retrieval` |
| Durable search | `text-index` memory/SQLite Text Index | Optional caller-supplied embedders; hashed embeddings by default |
| Embeddings | `HashedTextEmbedder` | Optional ONNX/Candle embedders in `text-embeddings` |
| Linguistic analysis | Heuristic pipeline in `text-linguistics` | Optional local sequence labeler for NER |
| Transcription | Transcript parsers | Whisper CLI/native whisper.cpp adapters |
| Classification/sentiment | `text-classification` lexical/imported fallbacks | `distilbert-sst2` through Candle sequence classification when `local-models` is enabled; caller-supplied sequence classifier backends remain supported |
| Question answering | `text-question-answering` imported span postprocessing and heuristic fallback | `onnx-community/roberta-base-squad2-ONNX` through `text-model-runtime` with `local-onnx`; runtime-broker traits supplied by callers |
| Reranking | `text-retrieval` ranking APIs | Runtime-broker traits supplied by callers |
| Zero-shot classification | `text-classification` lexical/imported label scoring | `xenova-bart-large-mnli-onnx` through ONNX pair/NLI scoring when `local-models` is enabled; caller-supplied pair classifier backends remain supported |
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

`text-index` owns the durable Text Index boundary. It is the home for canonical
generic ingestion from `TextDocumentContract`, `TextSegmentContract`,
`TextCorpusDocument`, and caller-supplied index records, plus deterministic
chunking, in-memory and SQLite-backed indexing, stored vectors, semantic facets,
metadata/source/time/provenance filters, and hybrid score explanations. SQLite
is feature-gated and uses bundled FTS5 when enabled.

`text-retrieval` is transitional. Its `RetrievalIndex` remains available as
soft-legacy compatibility for existing package consumers, but new durable
indexing/search work belongs in `text-index`. Retrieval should focus on
compatibility wrappers, `SearchDocument` adapters for older callers, persisted
retrieval snapshot import into `text-index`, and reranking.

`text-question-answering` uses `text-index` as the primary path for new cited
document QA. Package surfaces stay request-scoped: `qa.answerWithIndex` builds a
deterministic in-memory Text Index from the request and does not create
server-side sessions or open index handles. `qa.answerWithRetrieval` remains for
soft-legacy compatibility with older retrieval consumers.

`text-transcripts` owns transcript extensions such as
`TranscriptSegmentContract`, `TranscriptWordContract`, and
`TranscriptionContract`. A transcript is treated as timed text with optional
speaker, confidence, and word-level metadata, and it must convert into the
generic `text-core` segment contract for lexical, linguistic, retrieval, and
pipeline consumers.

Audio ASR/model crates should return `TranscriptionContract` rather than
defining their own transcript DTO. Compatibility structs should not be added to
new release surfaces; convert into the transcript contract at the boundary.
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
caller-supplied runtime backends, imported predictions, explicit local model
options, or explicit fallback policies. `text-classification` owns the mixed
classification policy: Candle sequence classification for ordinary
classification/sentiment, and ONNX pair/NLI classification for zero-shot.
`text-model-runtime` owns reusable tokenizer, Candle, and ONNX internals.
Reusable model acquisition belongs in `model-runtime`.

## Feature Policy

Default features are deterministic and network-free. Optional runtime features
may enable tokenizers, ONNX Runtime, or Candle, but callers must still select or
provide the runtime explicitly. Omitted download fields are treated according to
the operation contract: generic validation remains no-download, while
model-backed QA/classification package workflows may default to
`autoDownload: true` only when built with the relevant native/model features.
External tests that require real tools, models, or network access remain behind
`external-tests`.

Text Candle server binaries default to CPU. Native CUDA execution is opt-in with
the server `cuda` feature and startup flags:

```bash
cargo run -p moritzbrantner-text-analysis-server -- --addr 127.0.0.1:3000
cargo run -p moritzbrantner-text-analysis-server --features cuda -- --cuda --cuda-device-index 0
```

CUDA requires a CUDA-capable host and Candle CUDA build prerequisites. WASM
Candle surfaces remain CPU-only and must not enable the native `cuda` feature.
