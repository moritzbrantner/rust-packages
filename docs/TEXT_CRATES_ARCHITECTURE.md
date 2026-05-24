# Text Crates Architecture

The text crates are local-first. Default builds must not require network access,
model downloads, hosted AI credentials, or native inference runtimes. Model
execution is opt-in through feature flags and explicit runtime configuration.

## Responsibilities

| Crate | Owns | Must not own |
| --- | --- | --- |
| `text-core` | Text documents, Unicode-safe spans, tokenization, sentence and paragraph boundaries, annotation graph primitives. | Model downloads, native inference, corpus search, transcript formats, transport concerns. |
| `text-lexical` | Deterministic lexical features, stop words, keywords, TF-IDF, BM25, rule entities, extractive summaries, lexical sentiment. | ASR, transcript-specific source adapters, native model execution. |
| `text-model-runtime` | Shared tokenizer bundles, tokenized model inputs, runtime backend traits, and optional native model facade types. | High-level NLP schemas, retrieval indexes, transcript parsing, text pipeline orchestration. |
| `text-linguistics` | Structured NLP pipeline: language, lemmas, POS, morphology, syntax, entities, coreference, events, discourse, topics, style. | Generic task schemas, vector retrieval storage, transcript file formats. |
| `text-embeddings` | Embedding backends, pooling, hashed fallback vectors, semantic search indexes. | General text classification, transcript parsing, linguistic annotations. |
| `text-retrieval` | Chunking, metadata filters, BM25/vector/hybrid retrieval, persistence helpers. | Embedding model internals, ASR, linguistic parsing. |
| `text-transcripts` | Transcript formats, ASR command adapters, whisper.cpp integration, transcript-specific analyzers. | Generic lexical features, retrieval ranking. |
| `text-nlp-models` | Shared NLP task schemas, model catalog, imported-prediction handling, deterministic fallbacks, runtime broker APIs. | Tokenizer implementation details, direct download policy, high-level linguistics graph construction. |
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
| Classification/sentiment/rerank/QA | `text-nlp-models` lexical/imported fallbacks | Runtime-broker traits supplied by callers |
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

`text-nlp-models` brokers task-level behavior. It may accept caller-supplied
runtime backends, imported predictions, or explicit fallback policies. It should
not silently download models or make native inference mandatory.

## Feature Policy

Default features are deterministic and network-free. Optional runtime features
may enable tokenizers, ONNX Runtime, or Candle, but callers must still select or
provide the runtime explicitly. External tests that require real tools, models,
or network access remain behind `external-tests`.
