# text-transcripts

Transcript parsing and ASR command adapters for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored CLI-backed smoke tests

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
