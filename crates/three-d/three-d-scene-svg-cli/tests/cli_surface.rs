#[test]
fn cli_adapter_reports_wrapped_library() {
    assert_eq!(three_d_scene_svg_cli::LIBRARY_CRATE, "three-d-scene-svg");
    assert_eq!(three_d_scene_svg_cli::SURFACE_KIND, "cli");
}
