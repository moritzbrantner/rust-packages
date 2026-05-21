#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_radiance_io_cli::LIBRARY_CRATE,
        "video-analysis-radiance-io"
    );
    assert_eq!(video_analysis_radiance_io_cli::SURFACE_KIND, "cli");
}
