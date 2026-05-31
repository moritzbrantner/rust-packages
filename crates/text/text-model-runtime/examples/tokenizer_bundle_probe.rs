use text_model_runtime::surface::run_surface_operation;
use video_analysis_core::runtime::{OperationId, SurfaceRequest};

fn main() {
    let response = run_surface_operation(SurfaceRequest {
        operation: OperationId::new("runtime.tokenizeSummary"),
        input: serde_json::json!({
            "text": "Rust text runtime can inspect tokenizer-shaped inputs without downloads.",
            "maxTokens": 10
        }),
    })
    .expect("tokenizer summary");

    println!("{}", serde_json::to_string_pretty(&response.value).unwrap());
}
