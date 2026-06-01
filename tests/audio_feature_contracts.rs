use runtime_core::{OperationId, SurfaceRequest};
use video_analysis as va;

#[test]
fn feature_series_flow_across_core_fourier_and_processing() {
    let samples = serde_json::json!([0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0]);

    let levels = va::audio_core::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.levels"),
        input: serde_json::json!({
            "samples": samples,
            "sampleRate": 8000,
            "channels": 1,
            "frameSize": 4,
            "hopSize": 2
        }),
    })
    .expect("levels");
    assert_eq!(levels.value["featureSeries"]["sample_rate"], 8000);
    assert_eq!(levels.value["featureSummary"]["frame_count"], 3);

    let fourier = va::audio_fourier::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.fourier.features"),
        input: serde_json::json!({
            "samples": [0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0],
            "sampleRate": 8000,
            "fftSize": 4,
            "hopSize": 2,
            "melBandCount": 3
        }),
    })
    .expect("fourier features");
    assert!(fourier.value["frames"].as_array().unwrap().len() >= 3);
    assert_eq!(fourier.value["melBands"].as_array().unwrap().len(), 3);

    let loudness = va::audio_processing::surface::run_surface_operation(SurfaceRequest {
        operation: OperationId::new("audio.processing.loudness"),
        input: serde_json::json!({
            "samples": [0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0],
            "sampleRate": 8000,
            "channels": 1,
            "frameSize": 4,
            "hopSize": 2
        }),
    })
    .expect("loudness");
    assert_eq!(loudness.value["frameSeries"]["sample_rate"], 8000);
    assert!(loudness.value["approximateLufs"]
        .as_f64()
        .unwrap()
        .is_finite());
}
