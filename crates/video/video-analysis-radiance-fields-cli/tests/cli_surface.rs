#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        video_analysis_radiance_fields_cli::LIBRARY_CRATE,
        "video-analysis-radiance-fields"
    );
    let surface = video_analysis_radiance_fields_cli::package_surface();
    assert_eq!(surface.library, "video-analysis-radiance-fields");
    assert!(!surface.operations.is_empty());
}
