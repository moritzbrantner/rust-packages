#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(
        data_inversion_core_cli::LIBRARY_CRATE,
        "data-inversion-core"
    );
    assert_eq!(data_inversion_core_cli::SURFACE_KIND, "cli");
}
