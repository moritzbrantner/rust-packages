# text-transcripts

Transcript parsing, ASR command adapters, and native whisper.cpp support for `moritzbrantner-video-analysis`.

## Feature flags

- `external-tests`: enables ignored CLI-backed smoke tests
- `native`: builds whisper.cpp support for offline transcription. Repository
  builds use `vendor/whisper.cpp`; crates.io builds must set
  `WHISPER_CPP_SOURCE_DIR` to a local whisper.cpp source checkout.

## Stable contract

The stable surface is transcript contracts, segment/word normalization,
SRT/WebVTT/plain/Whisper JSON parsing, formatting, conversion to
`TextSegmentContract`, and transcript-specific text pipeline analyzers.

## Quality and limits

Default package operations parse and format text only. ASR command adapters and
native whisper.cpp transcription remain explicit runtime paths and are not
invoked by default package-surface operations.

## Example

```rust,no_run
use text_transcripts::{parse_whisper_json, TranscriptionContract};

let parsed = parse_whisper_json(include_bytes!("../../../../tests/fixtures/whisper-sample.json"))?;
let transcript = TranscriptionContract::from(parsed).normalized()?;

assert!(!transcript.text_or_joined().is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Package surface

- Primary workflow: `transcripts.parse` parses plain text, Whisper JSON, SRT, or
  WebVTT into the normalized transcript contract.
- Workflow operations: `transcripts.parse`, `transcripts.normalize`,
  `transcripts.formatSrt`, `transcripts.formatWebVtt`, and
  `transcripts.toTextSegments`.
- Debug operations: `describe` inspects package metadata and operation support.
- Runtime support: pure Rust parsing/formatting package-surface operations are
  available through library, CLI, server, and WASM wrappers.
- Sample output includes `title`, `message`, `summary`, `result`, and
  operation-specific fields such as `segments`, `text`, `srt`, or `webVtt`.
- Package-surface operations do not invoke whisper.cpp or external ASR tools;
  native transcription remains feature-gated.

## Native whisper.cpp

The transcript parsers are loadable in default builds. whisper.cpp catalog and
model-store validation is available behind `native`; transcription only runs
when the requested model file is present or an opt-in setup flow downloads it.

```bash
cargo test -p text-transcripts --features native,external-tests -- --ignored
```

Browser benchmarks cover parse, normalize, and SRT formatting workflows through
`bun run text-wasm:bench:all`.

## Related crates

- `text-core`
- `video-analysis-ingest`
- `video-analysis-use-cases`
