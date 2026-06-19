# ADR 0010: Speaker-Conditioned TTS Gets Its Own Crate Family

## Status

Accepted.

## Context

The workspace already has deterministic audio synthesis and MIDI-like audio
generation crates. Those crates turn symbolic events, tones, timelines, pitch
tracks, and MIDI-like note data into deterministic audio outputs. They are
useful package-consumer building blocks, but they do not own speaker prompts,
model-bundle selection, native inference planning, or license-sensitive model
metadata.

Speaker-conditioned TTS has a different contract. Package consumers need to
name target text, Reference Voice Prompt inputs, transcript-present and
transcript-missing paths, TTS Provider setup, device planning, model bundles,
and explicit model presets before any native F5, E2, or Vocos inference exists.
Default contributor builds must remain side-effect free: no native model
downloads, CUDA setup, network access, or silent model preset selection.

Consent, identity, and safety policy are product concerns. This repository can
expose stable request fields and diagnostics that downstream products use to
enforce policy, but it should not encode product-specific consent or safety
rules into core crate contracts.

## Decision

Create a new `audio-generation-tts` crate family for speaker-conditioned TTS
instead of extending deterministic audio synthesis.

The new crate family owns:

- speaker-conditioned TTS request and response contracts;
- Reference Voice Prompt planning and validation language;
- TTS Provider planning and unsupported-runtime diagnostics;
- package-surface operations such as `audio.tts.synthesize`,
  `audio.tts.plan`, `audio.tts.models`, and
  `audio.tts.referencePromptPlan`;
- explicit F5, E2, and Vocos model preset metadata, including repo ids,
  required files, and visible license metadata.

Default builds remain model-free, network-free, and no-download. Native
providers, model bundle materialization, CUDA use, and F5/E2/Vocos inference
must require explicit feature gates or explicit model-bundle choices. F5 and E2
presets are explicit opt-in because their model licenses and distribution terms
are materially different from deterministic audio synthesis code.

Consent and safety policy is downstream-owned. The TTS contracts may carry
fields and diagnostics that make policy enforcement possible, but they do not
decide whether a caller is allowed to synthesize a particular voice.

## Consequences

Deterministic audio synthesis crates stay focused on symbolic and predictable
audio generation.

Speaker-conditioned TTS can evolve around model bundle validation, reference
prompt preparation, provider diagnostics, and native debug operations without
pulling native runtime concerns into existing deterministic crates.

Package consumers get stable TTS domain language before native synthesis is
implemented. Early package-surface operations can validate requests and return
clear setup or unsupported-runtime responses while later slices add model
presets, planning, diagnostics, and end-to-end native synthesis.
