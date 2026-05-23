#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        image_analysis_tasks_cli::LIBRARY_CRATE,
        "image-analysis-tasks"
    );
    let surface = image_analysis_tasks_cli::package_surface();
    assert_eq!(surface.library, "image-analysis-tasks");
    assert!(!surface.operations.is_empty());
}
