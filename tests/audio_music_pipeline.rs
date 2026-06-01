use runtime_core::{OperationId, SurfaceRequest};
use video_analysis as va;

#[test]
fn pitch_track_midi_and_click_track_flow() {
    let chroma = va::audio_pitch::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.pitch.chroma"),
        input: serde_json::json!({
            "samples": (0..800).map(|index| {
                let t = index as f32 / 8000.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin()
            }).collect::<Vec<_>>(),
            "sampleRate": 8000
        }),
    })
    .expect("chroma");
    assert_eq!(chroma.value["strongestPitchClass"], "A");

    let midi = va::audio_midi::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.midi.fromPitchTrack"),
        input: serde_json::json!({
            "tempoBpm": 120.0,
            "pitchTrack": [
                {"startSeconds": 0.0, "endSeconds": 0.5, "frequencyHz": 440.0, "confidence": 0.9},
                {"startSeconds": 0.5, "endSeconds": 1.0, "frequencyHz": 440.0, "confidence": 0.9}
            ]
        }),
    })
    .expect("pitch track to midi");
    assert_eq!(midi.value["noteCount"], 1);
    assert_eq!(midi.value["notes"][0]["note"], 69);

    let click = va::audio_synthesis::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.synthesis.clickTrack"),
        input: serde_json::json!({
            "beatGrid": [0.0, 0.5],
            "durationSeconds": 1.0,
            "sampleRate": 1000
        }),
    })
    .expect("click track");
    assert_eq!(click.value["sampleCount"], 1000);
    assert_eq!(click.value["beatCount"], 2);
}
