#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_gaussian_splatting_cli::LIBRARY_CRATE,
        "video-analysis-gaussian-splatting"
    );
    let surface = video_analysis_gaussian_splatting_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-gaussian-splatting");
    assert!(!surface.operations.is_empty());
}
