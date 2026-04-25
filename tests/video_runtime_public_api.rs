mod support;

use std::f32::consts::FRAC_PI_3;

use support::scene;
use tempfile::tempdir;
use video_analysis_core::BoundingBox;
use video_analysis_ffmpeg::{is_ffmpeg_available, FfmpegAudioSourceOptions, FfmpegSourceOptions};
use video_analysis_gaussian_splatting::{
    project_scene, render_projected_splats, Gaussian3d, GaussianScene, ProjectionConfig,
    SplatRenderConfig,
};
use video_analysis_models::{
    normalize_predictions, ModelPreset, ModelTask, PredictionRepairOptions, RawBoundingBox,
    RawPrediction,
};
use video_analysis_radiance_fields::{CameraIntrinsics, CameraPose, ColorRgb, Vec2, Vec3};
use video_analysis_reconstruction::{match_binary_features, BinaryFeature, Feature2d, MatchConfig};
use video_analysis_segmentation::{
    default_sam2_model_spec, VideoSegmentationPrompt, VideoSegmentationRequest,
};
use video_analysis_split::{build_split_plan, SplitOptions};
use video_analysis_synthesis::{
    frame_from_spec, FrameSynthesisSpec, RgbColor, SynthesizedRegion, VideoSynthesisConfig,
};

#[test]
fn runtime_and_projection_crates_expose_hermetic_public_surfaces(
) -> Result<(), Box<dyn std::error::Error>> {
    let recorded = FfmpegSourceOptions::recorded().extra_input_arg("-nostdin");
    let audio = FfmpegAudioSourceOptions::live().samples_per_chunk(0);
    assert!(!recorded.realtime);
    assert_eq!(audio.samples_per_chunk, 1);
    let _ = is_ffmpeg_available();

    let preset = ModelPreset::YolosTiny.spec();
    assert_eq!(
        preset.task,
        video_analysis_models::ModelTask::ObjectDetection
    );
    let normalized = normalize_predictions(
        vec![RawPrediction::object(
            "person",
            0.9,
            RawBoundingBox::xywh(0.0, 0.0, 2.0, 2.0),
        )],
        &ModelTask::ObjectDetection,
        Some((4, 4)),
        PredictionRepairOptions::default(),
    );
    assert_eq!(normalized[0].label.as_deref(), Some("person"));

    let prompt = VideoSegmentationPrompt::default().object_id("person-1");
    let request = VideoSegmentationRequest::default().min_mask_pixels(16);
    assert_eq!(prompt.object_id.as_deref(), Some("person-1"));
    assert_eq!(request.min_mask_pixels, 16);
    assert_eq!(
        default_sam2_model_spec().task,
        video_analysis_models::ModelTask::Custom("video_segmentation".to_string())
    );

    let split_plan = build_split_plan("demo.mp4", &[scene(0, 15)], &SplitOptions::default())?;
    assert_eq!(split_plan.jobs.len(), 1);

    let intrinsics = CameraIntrinsics::pinhole(32, 32, FRAC_PI_3)?;
    let pose = CameraPose::identity();
    let gaussian = Gaussian3d::isotropic(
        Vec3::new(0.0, 0.0, 3.0),
        0.5,
        ColorRgb::new(1.0, 0.0, 0.0),
        0.8,
    )?;
    let projected = project_scene(
        &GaussianScene::new([gaussian])?,
        intrinsics,
        pose,
        ProjectionConfig::default(),
    )?;
    let pixels = render_projected_splats(&projected, SplatRenderConfig::new(32, 32)?)?;
    assert_eq!(pixels.len(), 32 * 32);

    let frame = frame_from_spec(
        &FrameSynthesisSpec::new(0, RgbColor::new(16, 18, 20)).region(SynthesizedRegion {
            region: BoundingBox::new(2, 2, 4, 4)?,
            color: RgbColor::new(255, 0, 0),
        }),
        VideoSynthesisConfig::new(16, 16, num_rational::Rational64::new(24, 1))?,
    )?;
    assert_eq!(frame.value.width, 16);

    let left = [BinaryFeature::new(
        Feature2d::new(Vec2::new(0.0, 0.0))?,
        [0b1010_1010],
    )?];
    let right = [BinaryFeature::new(
        Feature2d::new(Vec2::new(1.0, 0.0))?,
        [0b1010_1010],
    )?];
    let matches = match_binary_features(&left, &right, MatchConfig::default())?;
    assert_eq!(matches.len(), 1);

    let _ = tempdir()?;
    Ok(())
}
