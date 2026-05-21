#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_gaussian_splatting_cli::LIBRARY_CRATE,
        "video-analysis-gaussian-splatting"
    );
    assert_eq!(video_analysis_gaussian_splatting_cli::SURFACE_KIND, "cli");
}
