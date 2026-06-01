use runtime_core::{OperationId, SurfaceRequest};
use video_analysis as va;

#[test]
fn vad_diarization_and_transcript_assignment_flow() {
    let vad = va::audio_speakers::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.speakers.vad"),
        input: serde_json::json!({
            "samples": [0.0, 0.2, 0.2, 0.0, 0.0, 0.3, 0.3, 0.0],
            "sampleRate": 4,
            "channels": 1,
            "frameSize": 2,
            "hopSize": 1,
            "threshold": 0.01,
            "minSpeechSeconds": 0.0,
            "minSilenceSeconds": 0.0
        }),
    })
    .expect("vad");
    assert!(!vad.value["segments"].as_array().unwrap().is_empty());

    let diarize = va::audio_speakers::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.speakers.diarize"),
        input: serde_json::json!({
            "importedSegments": [
                {"speaker": "speaker_a", "startSeconds": 0.0, "endSeconds": 1.0, "score": 0.9},
                {"speaker": "speaker_b", "startSeconds": 1.0, "endSeconds": 2.0, "score": 0.8}
            ]
        }),
    })
    .expect("diarize");
    assert_eq!(diarize.value["speakerCount"], 2);

    let assigned = va::audio_speakers::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.speakers.assignTranscript"),
        input: serde_json::json!({
            "overlapPolicy": "majority",
            "transcript": {
                "segments": [
                    {"index": 0, "text": "hello", "startSeconds": 0.0, "endSeconds": 1.0, "isFinal": true},
                    {"index": 1, "text": "world", "startSeconds": 1.0, "endSeconds": 2.0, "isFinal": true}
                ]
            },
            "diarization": diarize.value["result"]
        }),
    })
    .expect("assign transcript");

    assert_eq!(assigned.value["segments"][0]["speaker"], "speaker_a");
    assert_eq!(assigned.value["segments"][1]["speaker"], "speaker_b");
}
