# audio-generation-tts

`audio-generation-tts` owns the first stable package-consumer contract for
generic text-to-speech and speaker-conditioned TTS.

The primary workflow operation is `audio.tts.synthesize`. It validates the
request and returns explicit setup/unsupported-runtime diagnostics instead of
running native model inference. Native F5/E2/Vocos providers remain opt-in and
are added by later slices.

Package-surface operations:

- `audio.tts.synthesize` validates a synthesis request and returns a
  side-effect-free setup response.
- `audio.tts.plan` previews provider, runtime, and output requirements.
- `audio.tts.models` reports the current model inventory state.
- `audio.tts.referencePromptPlan` inspects Reference Voice Prompt readiness.
- `describe` returns package and operation metadata.

Native planning is side-effect free. `audio.tts.plan` can explain an explicit
`provider.modelId`, optional `provider.modelBundle.bundlePath`,
`provider.modelBundle.autoDownload`, and `provider.modelBundle.cacheOnly`
without resolving files, downloading models, probing hardware, or running
inference. Cache-only mode forbids downloads even when `autoDownload` is set.
Model download planning is only allowed when the crate is built with the
explicit `model-bundles` feature; default builds report the requirement instead
of materializing files.

Device planning uses `provider.device`:

- `auto` is the default and is CUDA-preferred when the crate is built with
  `cuda` and a CUDA device is available; otherwise native providers should use
  CPU.
- `cpu` requests CPU execution.
- `cuda` requests CUDA execution and requires the `cuda` feature plus an
  available CUDA device in later native providers.

Cargo features are explicit and default to off: `candle`, `cuda`,
`model-bundles`, `audio-io`, `asr`, and `external-tests`.
