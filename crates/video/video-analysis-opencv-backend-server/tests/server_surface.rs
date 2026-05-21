#[test]
fn package_endpoint_reports_wrapped_library() {
    let response = video_analysis_opencv_backend_server::response_for("GET", "/api/package", "");
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("video-analysis-opencv-backend"));
}
