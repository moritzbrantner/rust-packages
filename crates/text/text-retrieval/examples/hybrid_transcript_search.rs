use text_retrieval::surface::run_surface_operation;
use video_analysis_core::runtime::{OperationId, SurfaceRequest};

fn main() {
    let response = run_surface_operation(SurfaceRequest {
        operation: OperationId::new("retrieval.search"),
        input: serde_json::json!({
            "documents": [
                {"id": "clip-1", "body": "Alice explains Rust text retrieval in the intro."},
                {"id": "clip-2", "body": "The demo shows scene boundaries and captions."},
                {"id": "clip-3", "body": "Transcript search combines lexical and semantic signals."}
            ],
            "query": "text retrieval transcript",
            "mode": "hybrid"
        }),
    })
    .expect("hybrid search");

    println!("{}", serde_json::to_string_pretty(&response.value).unwrap());
}
