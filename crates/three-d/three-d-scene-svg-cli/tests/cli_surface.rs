#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(three_d_scene_svg_cli::LIBRARY_CRATE, "three-d-scene-svg");
    let surface = three_d_scene_svg_cli::package_surface();
    assert_eq!(surface.library, "moenarch-three-d-scene-svg");
    assert!(!surface.operations.is_empty());
}
