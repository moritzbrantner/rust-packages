use runtime_core::{OperationId, SurfaceRequest};
use video_analysis as va;

#[test]
fn audio_surfaces_expose_cross_crate_deterministic_flow() {
    let samples = serde_json::json!([0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0]);

    let pitch = va::audio_pitch::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.pitch.noteName"),
        input: serde_json::json!({"frequencyHz": 440.0}),
    })
    .expect("pitch");
    assert_eq!(pitch.value["noteName"], "A4");

    let spectrum = va::audio_fourier::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.fourier.spectrum"),
        input: serde_json::json!({"samples": samples, "sampleRate": 8000, "fftSize": 128}),
    })
    .expect("spectrum");
    assert!(spectrum.value["binCount"].as_u64().unwrap() > 0);

    let rhythm = va::audio_rhythm::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.rhythm.beatGrid"),
        input: serde_json::json!({"startSeconds": 0.0, "bpm": 120.0, "beats": 4}),
    })
    .expect("beat grid");
    assert_eq!(rhythm.value["grid"].as_array().unwrap().len(), 4);

    let synthesis = va::audio_synthesis::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.synthesis.tone"),
        input: serde_json::json!({"frequencyHz": 440.0, "durationSeconds": 0.01, "sampleRate": 8000}),
    })
    .expect("synthesis");
    assert!(synthesis.value["sampleCount"].as_u64().unwrap() > 0);

    let levels = va::audio_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.levels"),
        input: serde_json::json!({"samples": [0.0, 1.0, -1.0, 0.0], "sampleRate": 8000, "channels": 1, "frameSize": 2, "hopSize": 1}),
    })
    .expect("levels");
    assert_eq!(levels.value["featureSummary"]["frame_count"], 3);

    let loudness = va::audio_processing::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.processing.loudness"),
        input: serde_json::json!({"samples": [0.0, 1.0, -1.0, 0.0], "sampleRate": 8000, "channels": 1, "frameSize": 2, "hopSize": 1}),
    })
    .expect("loudness");
    assert!(loudness.value["approximateLufs"]
        .as_f64()
        .unwrap()
        .is_finite());

    let midi = va::audio_midi::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.midi.render"),
        input: serde_json::json!({
            "tempoBpm": 120.0,
            "sampleRate": 8000,
            "notes": [{"note": 69, "startBeats": 0.0, "durationBeats": 0.25}]
        }),
    })
    .expect("midi render");
    assert!(midi.value["sampleCount"].as_u64().unwrap() > 0);

    let tts = va::audio_tts::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.tts.synthesize"),
        input: serde_json::json!({"text": "Hello from the facade."}),
    })
    .expect("tts synthesize");
    assert_eq!(tts.value["result"]["status"], "unsupportedRuntime");
    assert_eq!(tts.value["result"]["audioGenerated"], false);
}
