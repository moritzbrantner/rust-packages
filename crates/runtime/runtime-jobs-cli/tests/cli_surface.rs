#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(runtime_jobs_cli::LIBRARY_CRATE, "runtime-jobs");
    let surface = runtime_jobs_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-runtime-jobs");
    assert!(!surface.operations.is_empty());
}
