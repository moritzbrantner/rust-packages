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

## Related crates

- `audio-analysis-core`
- `audio-analysis-fourier`
- `text-transcripts`
