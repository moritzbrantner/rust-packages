use text_linguistics::surface::run_surface_operation;
use video_analysis_core::runtime::{OperationId, SurfaceRequest};

fn main() {
    let response = run_surface_operation(SurfaceRequest {
        operation: OperationId::new("linguistics.analyze"),
        input: serde_json::json!({
            "text": "Alice presented the tokenizer roadmap in Berlin. Bob linked the event to transcript search.",
            "profile": "rich"
        }),
    })
    .expect("linguistic analysis");

    println!("{}", serde_json::to_string_pretty(&response.value).unwrap());
}
