# audio-analysis-recognition

Deterministic audio embeddings, similarity search, and generic transcription
contracts for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_recognition::{
    transcribe, TranscriptSegmentContract, TranscriptionInput, TranscriptionRequest,
    TranscriptionRuntimeSelection,
};

let response = transcribe(TranscriptionRequest {
    source: Some("fixture.wav".to_string()),
    language: Some("en".to_string()),
    input: TranscriptionInput::ImportedSegments {
        segments: vec![TranscriptSegmentContract::new(0, "hello world")],
    },
    runtime: TranscriptionRuntimeSelection::default(),
})?;

assert_eq!(response.transcript.text_or_joined(), "hello world");
```

`SpeechRecognitionRequest` and `transcribe_audio` remain available as
compatibility shims for existing callers. New transcription integrations should
prefer `TranscriptionRequest`, `TranscriptionInput`, and `transcribe`.

Default builds do not run native ASR, download models, call network services, or
spawn external transcription commands. They normalize imported transcript
segments or transcript contracts. Native whisper.cpp execution remains owned by
`text-transcripts`; this crate exposes generic audio-facing transcription
contracts and runtime plans.

## Package surface

Primary workflow: `audio.recognition.embed`.

Workflow operations:

- `audio.recognition.embed`: Computes a deterministic spectral embedding for normalized samples.
- `audio.recognition.compare`: Compares two in-memory sample arrays by cosine similarity.
- `audio.recognition.search`: Builds a transient sample-backed reference library and searches it.
- `audio.recognition.transcribe`: Normalizes generic imported transcript input into the shared transcription contract without running native ASR.
- `audio.recognition.transcribeImported`: Normalizes caller-supplied transcript segments into the shared transcription contract without running native ASR.

Debug operations:

- `describe`: inspect package metadata and runtime support.
- `audio.recognition.transcriptionPlan`: Explains generic transcription provider setup without running native ASR, reading files, or writing outputs.
- `audio.recognition.transcriptionProviders`: Lists imported, Whisper, external, and model-runtime transcription provider support in default builds.

Runtime support: library, CLI, server, and WASM wrappers expose these operations.

Run the primary workflow through the CLI:

```bash
cargo run -p moritzbrantner-audio-analysis-recognition-cli -- run \
  --operation audio.recognition.embed \
  --json '{"bands":8,"sampleRate":48000,"samples":[0.0,1.0,0.0,-1.0]}'
```

Successful responses use the shared package-surface shape with `operation`,
`title`, `message`, `summary`, and `result`. Default surface calls are
deterministic, local-first, and do not download models, write persistent files,
or execute external tools unless an operation explicitly documents native or
external-tool execution.

## Related crates

- `audio-analysis-core`
- `audio-analysis-fourier`
- `text-transcripts`
