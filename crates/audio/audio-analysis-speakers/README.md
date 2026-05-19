# audio-analysis-speakers

Speaker-domain APIs for `video-analysis`.

This crate keeps `audio-analysis-recognition` focused on generic embeddings and reference search while adding speaker-specific concepts:

- speaker IDs and labels
- model-versioned speaker embeddings
- speaker profiles and library snapshots
- enrollment and thresholded identification
- baseline energy VAD
- diarization traits and a simple VAD/window/cluster diarizer

`SpectralSpeakerEmbedder` is a deterministic baseline intended for tests and prototypes. It is not production-grade speaker verification. Production systems should use a model-backed embedder such as ECAPA-TDNN, x-vector, pyannote-style, or SpeechBrain-compatible speaker verification models.

## Feature flags

- No optional feature flags today.
