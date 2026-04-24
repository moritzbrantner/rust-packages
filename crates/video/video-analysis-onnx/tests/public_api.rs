use std::collections::BTreeMap;
use std::fs;

use num_rational::Rational64;
use tempfile::{tempdir, TempDir};
use video_analysis_core::{FramePosition, PixelFormat, VideoFrame};
use video_analysis_models::{DownloadedModel, HuggingFaceModelSpec, ModelBundleStore, ModelTask};
use video_analysis_onnx::{
    options_from_bundle, preprocess_frame, validate_onnx_vision_bundle, ChannelOrder,
    OnnxImagePreprocessing,
};

fn materialized_bundle(
) -> Result<(TempDir, video_analysis_models::ModelBundle), Box<dyn std::error::Error>> {
    let temp = tempdir()?;
    let source_dir = temp.path().join("download");
    fs::create_dir_all(&source_dir)?;
    let config_path = source_dir.join("config.json");
    let model_path = source_dir.join("model.onnx");
    fs::write(
        &config_path,
        br#"{"id2label":{"0":"background","1":"person"}}"#,
    )?;
    fs::write(&model_path, b"onnx")?;

    let spec = HuggingFaceModelSpec::new("fixture/onnx", ModelTask::ObjectDetection)
        .name("fixture-onnx")
        .file("config.json")
        .file("model.onnx");
    let bundle =
        ModelBundleStore::new(temp.path().join("bundles")).materialize(&DownloadedModel {
            spec,
            files: BTreeMap::from([
                ("config.json".to_string(), config_path),
                ("model.onnx".to_string(), model_path),
            ]),
        })?;
    Ok((temp, bundle))
}

#[test]
fn onnx_public_api_validates_bundles_and_preprocesses_frames(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, bundle) = materialized_bundle()?;
    let info = validate_onnx_vision_bundle(&bundle)?;
    assert!(info.model_path.ends_with("model.onnx"));

    let options = options_from_bundle(&bundle)?;
    assert_eq!(options.labels.get(&1).map(String::as_str), Some("person"));

    let bytes = vec![255_u8; 2 * 2 * 3];
    let frame = VideoFrame::packed(
        FramePosition::from_frame_index(0, Rational64::new(1, 1)),
        2,
        2,
        PixelFormat::Rgb24,
        &bytes,
        6,
    )?;
    let tensor = preprocess_frame(
        &frame,
        &OnnxImagePreprocessing {
            input_width: 2,
            input_height: 2,
            channel_order: ChannelOrder::Rgb,
            ..OnnxImagePreprocessing::default()
        },
    )?;
    assert_eq!(tensor.values.len(), 12);
    Ok(())
}
