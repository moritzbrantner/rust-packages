#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = video_analysis_colmap_backend_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("video-analysis-colmap-backend"));
}

#[test]
fn run_endpoint_calls_library_surface() {
    let response = video_analysis_colmap_backend_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"describe","input":{"includeOperations":true}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains(r#""operation""#));
}

#[test]
fn reconstruct_video_dispatch_uses_native_runner() {
    let response = video_analysis_colmap_backend_server::response_for(
        "POST",
        "/api/run",
        r#"{"operation":"video.colmap.reconstructVideo","input":{"videoPath":"prototypes/web/video-analysis-web/public/samples/video/missing-test-video.mp4"}}"#,
    );
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("invalid_request"));
    assert!(response.body.contains("not readable"));
}
