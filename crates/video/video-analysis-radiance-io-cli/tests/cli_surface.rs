#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_radiance_io_cli::LIBRARY_CRATE,
        "video-analysis-radiance-io"
    );
    let surface = video_analysis_radiance_io_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-radiance-io");
    assert!(!surface.operations.is_empty());
}
