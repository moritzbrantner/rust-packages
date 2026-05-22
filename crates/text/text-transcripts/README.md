# text-transcripts

Transcript parsing, ASR command adapters, and native whisper.cpp support for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored CLI-backed smoke tests
- `native`: builds bundled whisper.cpp support for offline transcription

## Example

```rust,ignore
use text_transcripts::{parse_whisper_json, WhisperCommandTranscriber};

let segments = parse_whisper_json(include_str!("../../../../tests/fixtures/whisper-sample.json"))?;
let transcriber = WhisperCommandTranscriber::default();

let _ = segments;
let _ = transcriber;
```

## Related crates

- `text-core`
- `video-analysis-ingest`
- `video-analysis-use-cases`
