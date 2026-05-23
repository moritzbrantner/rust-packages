#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(video_analysis_mvs_cli::LIBRARY_CRATE, "video-analysis-mvs");
    let surface = video_analysis_mvs_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-mvs");
    assert!(!surface.operations.is_empty());
}
