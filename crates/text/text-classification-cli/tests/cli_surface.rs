#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        text_classification_cli::LIBRARY_CRATE,
        "text-classification"
    );
    let surface = text_classification_cli::package_surface();
    assert_eq!(surface.library, "text-classification");
    assert!(!surface.operations.is_empty());
}
