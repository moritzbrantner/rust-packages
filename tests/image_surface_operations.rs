use runtime_core::{OperationId, SurfaceRequest};

fn run(
    operation: &str,
    input: serde_json::Value,
    runner: fn(SurfaceRequest) -> Result<runtime_core::SurfaceResponse, String>,
) -> serde_json::Value {
    runner(SurfaceRequest {
        operation: OperationId::new(operation),
        input,
    })
    .expect(operation)
    .value
}

#[test]
fn image_catalog_and_import_surfaces_accept_valid_inputs() {
    assert_eq!(
        run(
            "image.classification.imported",
            serde_json::json!({"labels": [{"label": "cat", "score": 0.7}]}),
            image_analysis_classification::surface::run_surface_operation,
        )["count"],
        1
    );
    assert_eq!(
        run(
            "image.captioning.imported",
            serde_json::json!({"captions": [{"text": "a cat", "score": 0.8}]}),
            image_analysis_captioning::surface::run_surface_operation,
        )["count"],
        1
    );
    assert_eq!(
        run(
            "image.embeddings.validate",
            serde_json::json!({"kind": "image", "vector": [0.1, 0.2]}),
            image_analysis_embeddings::surface::run_surface_operation,
        )["dimensions"],
        2
    );
}

#[test]
fn image_io_ocr_segmentation_and_synthesis_surfaces_accept_valid_inputs() {
    assert_eq!(
        run(
            "image.io.plan",
            serde_json::json!({"path": "image.png", "operation": "read"}),
            image_analysis_io::surface::run_surface_operation,
        )["format"],
        "png"
    );
    assert_eq!(
        run(
            "image.ocr.documentSummary",
            serde_json::json!({"text": "Hello", "width": 4, "height": 3}),
            image_analysis_ocr::surface::run_surface_operation,
        )["textLength"],
        5
    );
    assert_eq!(
        run(
            "image.segmentation.maskSummary",
            serde_json::json!({"width": 4, "height": 3, "rect": {"x": 1, "y": 1, "width": 2, "height": 1}}),
            image_analysis_segmentation::surface::run_surface_operation,
        )["activePixels"],
        2
    );
    assert_eq!(
        run(
            "image.synthesis.gradient",
            serde_json::json!({"width": 2, "height": 2, "top": {"red": 0, "green": 0, "blue": 0}, "bottom": {"red": 255, "green": 255, "blue": 255}}),
            image_analysis_synthesis::surface::run_surface_operation,
        )["dimensions"]["height"],
        2
    );
}

#[test]
fn image_detection_surface_summarizes_boxes() {
    let value = run(
        "image.detection.boxSummary",
        serde_json::json!({"detections": [{"label": "object", "score": 1.0, "region": {"x": 0, "y": 1, "width": 2, "height": 3}}]}),
        image_analysis_detection::surface::run_surface_operation,
    );
    assert_eq!(value["count"], 1);
    assert_eq!(value["unionBounds"]["height"], 3);
}

#[test]
fn image_processing_surface_accepts_seeded_noise_operations() {
    let image = serde_json::json!({
        "width": 2,
        "height": 2,
        "pixelFormat": "rgb24",
        "stride": null,
        "data": [32, 64, 96, 128, 160, 192, 255, 255, 255, 0, 0, 0]
    });
    let value = run(
        "image.processing.pipeline",
        serde_json::json!({
            "image": image,
            "operations": [
                {"type": "blueNoise", "amount": 6, "seed": 3},
                {"type": "poissonNoise", "scale": 48.0, "seed": 4}
            ],
            "previewLimit": 12
        }),
        image_analysis_processing::surface::run_surface_operation,
    );
    assert_eq!(value["pixelFormat"], "rgb24");
    assert_eq!(value["dataLength"], 12);
    assert!(value["dataPreview"]
        .as_array()
        .is_some_and(|data| data.len() == 12));
}
