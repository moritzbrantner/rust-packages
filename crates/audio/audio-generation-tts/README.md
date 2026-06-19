# audio-generation-tts

`audio-generation-tts` owns the first stable package-consumer contract for
generic text-to-speech and speaker-conditioned TTS.

The primary workflow operation is `audio.tts.synthesize`. In this slice it
validates the request and returns explicit setup/unsupported-runtime diagnostics
instead of running native model inference. Native F5/E2/Vocos providers and
model presets are added by later slices.

Package-surface operations:

- `audio.tts.synthesize` validates a synthesis request and returns a
  side-effect-free setup response.
- `audio.tts.plan` previews provider, runtime, and output requirements.
- `audio.tts.models` reports the current model inventory state.
- `audio.tts.referencePromptPlan` inspects Reference Voice Prompt readiness.
- `describe` returns package and operation metadata.
