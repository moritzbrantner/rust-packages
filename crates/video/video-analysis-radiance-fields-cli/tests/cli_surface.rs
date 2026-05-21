#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_radiance_fields_cli::LIBRARY_CRATE,
        "video-analysis-radiance-fields"
    );
    assert_eq!(video_analysis_radiance_fields_cli::SURFACE_KIND, "cli");
}
