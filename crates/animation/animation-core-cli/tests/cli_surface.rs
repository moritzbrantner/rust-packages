#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(animation_core_cli::LIBRARY_CRATE, "animation-core");
    let surface = animation_core_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-animation-core");
    assert!(!surface.operations.is_empty());
}
