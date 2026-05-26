# text-transcripts

Transcript parsing, ASR command adapters, and native whisper.cpp support for `video-analysis`.

## Feature flags

- `external-tests`: enables ignored CLI-backed smoke tests
- `native`: builds whisper.cpp support for offline transcription. Repository
  builds use `vendor/whisper.cpp`; crates.io builds must set
  `WHISPER_CPP_SOURCE_DIR` to a local whisper.cpp source checkout.

## Example

```rust,ignore
use text_transcripts::{parse_whisper_json, TranscriptionContract};

let parsed = parse_whisper_json(include_bytes!("../../../../tests/fixtures/whisper-sample.json"))?;
let transcript = TranscriptionContract::from(parsed).normalized()?;

assert!(!transcript.text_or_joined().is_empty());
```

## Related crates

- `text-core`
- `video-analysis-ingest`
- `video-analysis-use-cases`
