# audio-generation-midi

MIDI-like note sequencing, Standard MIDI export, and deterministic audio rendering helpers for `moritzbrantner-video-analysis`.

## Feature flags

- No optional feature flags today.

## Example

```rust,ignore
use audio_generation_midi::{MidiNote, MidiNoteEvent, MidiSong, MidiTrack, NoteName};

let mut lead = MidiTrack::new("lead");
lead.push(MidiNoteEvent::new(
    MidiNote::from_name(NoteName::A, 4)?,
    0.0,
    1.0,
)?)?;

let song = MidiSong::new(120.0)?.with_track(lead)?;
let midi_bytes = song.to_midi_bytes()?;
let audio = song.render(Default::default())?;

let _ = (midi_bytes, audio);
# Ok::<(), video_analysis_core::DetectError>(())
```

## Related crates

- `audio-analysis-synthesis`
- `video-analysis-core`
