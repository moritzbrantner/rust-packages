use text_core::surface::run_surface_operation;
use video_analysis_core::runtime::{OperationId, SurfaceRequest};

fn main() {
    let response = run_surface_operation(SurfaceRequest {
        operation: OperationId::new("text.tokenize"),
        input: serde_json::json!({
            "text": "Alice tagged #東京 from Berlin. Rust keeps café, emoji 👍, and offsets intact.",
            "includeStats": true
        }),
    })
    .expect("tokenize unicode text");

    println!("{}", serde_json::to_string_pretty(&response.value).unwrap());
}
