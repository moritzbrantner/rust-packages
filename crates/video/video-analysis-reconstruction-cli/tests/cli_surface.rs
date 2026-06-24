#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_reconstruction_cli::LIBRARY_CRATE,
        "video-analysis-reconstruction"
    );
    let surface = video_analysis_reconstruction_cli::package_surface();
    assert_eq!(surface.library, "moenarch-video-analysis-reconstruction");
    assert!(!surface.operations.is_empty());
}
