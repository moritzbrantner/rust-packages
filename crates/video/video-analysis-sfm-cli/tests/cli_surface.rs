#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(video_analysis_sfm_cli::LIBRARY_CRATE, "video-analysis-sfm");
    let surface = video_analysis_sfm_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-video-analysis-sfm");
    assert!(!surface.operations.is_empty());
}
