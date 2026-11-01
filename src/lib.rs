#![doc = include_str!("../README.md")]

#[cfg(feature = "animation")]
/// Re-exports the animation core API.
pub use animation_core as animation;
#[cfg(feature = "comfyui")]
/// Re-exports the ComfyUI data API.
pub use comfyui_data;
#[cfg(feature = "comfyui")]
/// Re-exports the ComfyUI latents API.
pub use comfyui_latents;
#[cfg(feature = "comfyui")]
/// Re-exports the ComfyUI models API.
pub use comfyui_models;
#[cfg(feature = "comfyui")]
/// Re-exports the image ComfyUI API.
pub use image_analysis_comfyui as image_comfyui;
#[cfg(feature = "three-d")]
/// Re-exports the three d core API.
pub use three_d_processing_core as three_d_core;
#[cfg(feature = "three-d")]
/// Re-exports the three d I/O API.
pub use three_d_processing_io as three_d_io;
#[cfg(feature = "three-d")]
/// Re-exports the three d mesh API.
pub use three_d_processing_mesh as three_d_mesh;
#[cfg(feature = "three-d")]
/// Re-exports the SVG-inspired 3D scene API.
pub use three_d_scene_svg as three_d_scene;
#[cfg(feature = "spatial")]
/// Re-exports the gaussian splatting API.
pub use video_analysis_gaussian_splatting as gaussian_splatting;
#[cfg(feature = "spatial")]
/// Re-exports the MVS API.
pub use video_analysis_mvs as mvs;
#[cfg(feature = "spatial")]
/// Re-exports the radiance fields API.
pub use video_analysis_radiance_fields as radiance_fields;
#[cfg(feature = "spatial")]
/// Re-exports the radiance I/O API.
pub use video_analysis_radiance_io as radiance_io;
#[cfg(feature = "spatial")]
/// Re-exports the radiance pipeline API.
pub use video_analysis_radiance_pipeline as radiance_pipeline;
#[cfg(feature = "spatial")]
/// Re-exports the reconstruction API.
pub use video_analysis_reconstruction as reconstruction;
#[cfg(feature = "spatial")]
/// Re-exports the SfM API.
pub use video_analysis_sfm as sfm;

#[cfg(all(test, feature = "three-d"))]
mod tests {
    use super::*;

    #[test]
    fn facade_reexports_three_d_scene() {
        let scene = three_d_scene::SceneDocument::new(
            three_d_scene::SceneViewport::new(120, 80).unwrap(),
            three_d_scene::Camera::orthographic(
                three_d_core::Point3::new(2.0, 2.0, 2.0),
                three_d_core::Point3::new(0.0, 0.0, 0.0),
                4.0,
            )
            .unwrap(),
            three_d_scene::Node::point(three_d_core::Point3::new(0.0, 0.0, 0.0)),
        )
        .unwrap();
        assert!(three_d_scene::render_svg(&scene)
            .unwrap()
            .contains("<circle"));
    }
}
