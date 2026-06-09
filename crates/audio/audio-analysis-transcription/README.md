# audio-analysis-transcription

External audio and video transcription orchestration for `video-analysis`.

The first provider wraps an installed Python `whisperx` command. It keeps real
ASR execution explicit: callers pass a local audio or video path, a model, CPU
or CUDA device settings, optional alignment/diarization options, and an output
directory. WhisperX handles decoding, batched ASR, VAD preprocessing, alignment,
and optional pyannote diarization.

Transcript contracts, normalization, caption formatting, and WhisperX JSON
import remain owned by `text-transcripts`.

## Package Operations

- `audio.transcription.transcribe`: run real audio/video transcription through
  the selected provider. The default provider is `whisperx-command`.
- `audio.transcription.importWhisperX`: import existing WhisperX JSON without
  running external tools.
- `audio.transcription.providers`: inspect available provider families.
- `audio.transcription.plan`: describe runtime setup without execution.
- `describe`: inspect package metadata.

## Setup

Install and configure external tools outside the default test flow:

```bash
whisperx --help
ffmpeg -version
python -c 'import whisperx'
```

Diarization requires a Hugging Face token accepted by pyannote:

```bash
export HF_TOKEN=...
```

No default build or test downloads models, requires CUDA, or requires network
access.
