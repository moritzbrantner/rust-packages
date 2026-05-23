#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        text_generation_linguistics_cli::LIBRARY_CRATE,
        "text-generation-linguistics"
    );
    let surface = text_generation_linguistics_cli::package_surface();
    assert_eq!(surface.library, "text-generation-linguistics");
    assert!(!surface.operations.is_empty());
}
