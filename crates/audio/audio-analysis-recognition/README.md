# audio-analysis-recognition

Deterministic audio embeddings, similarity search, and contract-first speech
recognition surfaces for `video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_analysis_recognition::{
    transcribe_audio, AudioRuntimeSelection, SpeechRecognitionRequest,
    TranscriptSegmentContract,
};

let response = transcribe_audio(SpeechRecognitionRequest {
    source: Some("fixture.wav".to_string()),
    language: Some("en".to_string()),
    model: AudioRuntimeSelection::default(),
    imported_segments: vec![TranscriptSegmentContract::new(0, "hello world")],
})?;

assert_eq!(response.text(), "hello world");
```

## Related crates

- `audio-analysis-core`
- `audio-analysis-fourier`
- `text-transcripts`
