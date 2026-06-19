# audio-generation-tts

`audio-generation-tts` owns the first stable package-consumer contract for
generic text-to-speech and speaker-conditioned TTS.

The primary workflow operation is `audio.tts.synthesize`. It validates the
request and returns explicit setup/unsupported-runtime diagnostics instead of
running native synthesis. The F5 mel diagnostic is opt-in and debug-only; full
F5/E2/Vocos synthesis remains owned by later slices.

Package-surface operations:

- `audio.tts.synthesize` validates a synthesis request and returns a
  side-effect-free setup response.
- `audio.tts.plan` previews provider, runtime, and output requirements.
- `audio.tts.models` reports the current model inventory state.
- `audio.tts.referencePromptPlan` inspects Reference Voice Prompt readiness.
- `audio.tts.debug.f5Mel` validates a local F5 bundle and returns a mel-level
  diagnostic without running a vocoder.
- `describe` returns package and operation metadata.

Native planning is side-effect free. `audio.tts.plan` can explain an explicit
`provider.modelId`, optional `provider.modelBundle.bundlePath`,
`provider.modelBundle.autoDownload`, and `provider.modelBundle.cacheOnly`
without resolving files, downloading models, probing hardware, or running
inference. Cache-only mode forbids downloads even when `autoDownload` is set.
Model download planning is only allowed when the crate is built with the
explicit `model-bundles` feature; default builds report the requirement instead
of materializing files.

Reference Voice Prompts accept either inline PCM samples or a caller-managed
path source. A prompt transcript must be non-empty when provided.
Speaker-conditioned synthesis requires either a transcript or an explicit
`referenceVoicePrompt.asrFallback` setup; default builds report missing ASR
fallback support as setup-required diagnostics. Building with `asr` enables
fallback planning through `audio-analysis-transcription` provider metadata, but
planning remains side-effect free and does not run ASR.

Device planning uses `provider.device`:

- `auto` is the default and is CUDA-preferred when the crate is built with
  `cuda` and a CUDA device is available; otherwise native providers should use
  CPU.
- `cpu` requests CPU execution.
- `cuda` requests CUDA execution and requires the `cuda` feature plus an
  available CUDA device in later native providers.

Cargo features are explicit and default to off: `candle`, `cuda`,
`model-bundles`, `audio-io`, `asr`, and `external-tests`.

`audio.tts.debug.f5Mel` requires an explicit local `bundlePath`. With `candle`
enabled it validates compatible F5 config, vocab, and safetensors files, then
allocates a constrained mel diagnostic tensor. It never downloads models,
emits PCM audio, or invokes Vocos.
