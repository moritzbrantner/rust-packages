#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(runtime_artifacts_cli::LIBRARY_CRATE, "runtime-artifacts");
    let surface = runtime_artifacts_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-runtime-artifacts");
    assert!(!surface.operations.is_empty());
}
