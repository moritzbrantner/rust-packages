use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use num_rational::Rational64;
use video_analysis_core::{
    BoundingBox, DetectError, FramePosition, ObservationKind, OwnedVideoFrame, PixelFormat, Result,
    TextAnalyzer, TextSegment, Timestamp, VideoAnalyzer, VideoFrame,
};

use super::*;

fn test_frame() -> OwnedVideoFrame {
    OwnedVideoFrame {
        position: FramePosition::from_frame_index(0, Rational64::new(30, 1)),
        width: 100,
        height: 50,
        pixel_format: PixelFormat::Rgb24,
        data: vec![0; 100 * 50 * 3],
        stride: 100 * 3,
    }
}

fn fake_downloaded_model(
    cache_dir: &Path,
    spec: HuggingFaceModelSpec,
    files: &[(&str, &str)],
) -> DownloadedModel {
    fs::create_dir_all(cache_dir).unwrap();
    let mut downloaded_files = BTreeMap::new();
    for (index, (remote_path, contents)) in files.iter().enumerate() {
        let local_path = cache_dir.join(format!("cache-file-{index}"));
        fs::write(&local_path, contents).unwrap();
        downloaded_files.insert((*remote_path).to_string(), local_path);
    }
    DownloadedModel {
        spec,
        files: downloaded_files,
    }
}

#[test]
fn preset_specs_include_weight_fallbacks() {
    let spec = ModelPreset::DetrResnet50.spec();
    assert_eq!(spec.repo_id, "facebook/detr-resnet-50");
    assert_eq!(spec.task, ModelTask::ObjectDetection);
    assert!(spec
        .files
        .iter()
        .any(|file| matches!(file, ModelFileRequest::FirstAvailable(_))));
}

#[test]
fn onnx_text_presets_request_xenova_files() {
    let classifier = ModelPreset::XenovaDistilbertSst2Onnx.spec();
    assert_eq!(
        classifier.repo_id,
        "Xenova/distilbert-base-uncased-finetuned-sst-2-english"
    );
    assert_eq!(classifier.task, ModelTask::TextClassification);
    assert!(classifier
        .files
        .contains(&ModelFileRequest::required("config.json")));
    assert!(classifier
        .files
        .contains(&ModelFileRequest::required("tokenizer.json")));
    assert!(classifier.files.iter().any(|file| matches!(
        file,
        ModelFileRequest::FirstAvailable(paths)
            if paths.iter().any(|path| path == "onnx/model_quantized.onnx")
    )));

    let embedder = ModelPreset::XenovaMiniLmL6V2Onnx.spec();
    assert_eq!(embedder.repo_id, "Xenova/all-MiniLM-L6-v2");
    assert_eq!(embedder.task, ModelTask::TextEmbedding);
    assert!(embedder.files.iter().any(|file| matches!(
        file,
        ModelFileRequest::FirstAvailable(paths)
            if paths.iter().any(|path| path == "onnx/model.onnx")
    )));
}

#[test]
fn audio_presets_register_huggingface_specs() {
    let classifier = ModelPreset::XenovaAstAudiosetOnnx.spec();
    assert_eq!(
        classifier.repo_id,
        "Xenova/ast-finetuned-audioset-10-10-0.4593"
    );
    assert_eq!(classifier.task, ModelTask::AudioClassification);
    assert!(classifier.files.iter().any(|file| matches!(
        file,
        ModelFileRequest::FirstAvailable(paths)
            if paths.iter().any(|path| path == "onnx/model_quantized.onnx")
    )));

    let asr = ModelPreset::WhisperTinyEn.spec();
    assert_eq!(asr.repo_id, "openai/whisper-tiny.en");
    assert_eq!(asr.task, ModelTask::SpeechRecognition);
    assert!(asr
        .files
        .contains(&ModelFileRequest::required("tokenizer.json")));

    let embedding = ModelPreset::ClapHtsatUnfused.spec();
    assert_eq!(embedding.task, ModelTask::AudioEmbedding);
}

#[test]
fn cuda_oxide_plan_records_runtime_contract() {
    let plan = ModelPreset::MiniLmL6V2
        .spec()
        .cuda_oxide_plan("text_embed_cuda", ["mean_pool_embeddings"])
        .runtime(
            CudaOxideRuntimeConfig::new()
                .device_index(1)
                .target_sm("sm_80"),
        );
    let attributes = plan.attributes();

    assert_eq!(plan.spec.repo_id, "sentence-transformers/all-MiniLM-L6-v2");
    assert_eq!(
        attributes.get("runtime.backend").map(String::as_str),
        Some("cuda_oxide")
    );
    assert_eq!(
        attributes
            .get("runtime.cuda.device_index")
            .map(String::as_str),
        Some("1")
    );
    assert_eq!(
        attributes.get("runtime.cuda.target_sm").map(String::as_str),
        Some("sm_80")
    );
    assert_eq!(
        attributes
            .get("runtime.cuda_oxide.module")
            .map(String::as_str),
        Some("text_embed_cuda")
    );
    assert_eq!(
        attributes
            .get("runtime.cuda_oxide.command")
            .map(String::as_str),
        Some("cargo oxide")
    );
    assert_eq!(
        attributes
            .get("runtime.cuda_oxide.docs")
            .map(String::as_str),
        Some(CUDA_OXIDE_BOOK_URL)
    );
}

#[test]
fn blue_green_prediction_check_accepts_cuda_oxide_candidate() {
    let green = vec![RawPrediction::label("POSITIVE", 0.875)];
    let mut blue_prediction = RawPrediction::label("POSITIVE", 0.87502);
    blue_prediction.attributes.extend(
        CudaOxideRuntimeConfig::new()
            .target_sm("sm_80")
            .attributes(),
    );
    let blue = vec![blue_prediction];

    let report = compare_blue_green_predictions(
        &green,
        &blue,
        BlueGreenPredictionTestOptions {
            max_score_delta: 0.001,
            compare_regions: true,
        },
    )
    .unwrap();

    assert_eq!(report.compared_predictions, 1);
    assert!(report.max_score_delta > 0.0);
}

#[test]
fn blue_green_prediction_check_rejects_drift() {
    let green = vec![RawPrediction::label("POSITIVE", 0.9)];
    let blue = vec![RawPrediction::label("NEGATIVE", 0.9)];

    let error =
        compare_blue_green_predictions(&green, &blue, BlueGreenPredictionTestOptions::default())
            .unwrap_err();

    assert!(matches!(error, DetectError::InvalidArgument(_)));
}

#[test]
fn model_bundle_store_materializes_files_and_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let downloaded = fake_downloaded_model(
        &dir.path().join("cache"),
        HuggingFaceModelSpec::new("owner/model", ModelTask::TextClassification)
            .name("test-model")
            .file("config.json")
            .file("model.safetensors"),
        &[
            ("config.json", "{\"model_type\":\"test\"}"),
            ("model.safetensors", "weights"),
        ],
    );

    let bundle = ModelBundleStore::new(dir.path().join("bundles"))
        .materialize(&downloaded)
        .unwrap();

    assert!(bundle.manifest_path().exists());
    for remote_path in ["config.json", "model.safetensors"] {
        let local_path = bundle.file_path(remote_path).unwrap();
        assert!(local_path.exists());
        assert!(fs::metadata(&local_path).unwrap().len() > 0);
    }

    let bundle_download = bundle.to_downloaded_model();
    assert_eq!(bundle_download.spec.name, "test-model");
    for remote_path in ["config.json", "model.safetensors"] {
        let local_path = &bundle_download.files[remote_path];
        assert!(local_path.is_absolute());
        assert!(local_path.starts_with(&bundle.root));
    }
}

#[cfg(unix)]
#[test]
fn model_bundle_store_materializes_symlinked_cache_files() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join("cache");
    let blobs_dir = cache_dir.join("blobs");
    let snapshots_dir = cache_dir.join("snapshots/main");
    fs::create_dir_all(&blobs_dir).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();

    let blob_path = blobs_dir.join("config-blob");
    fs::write(&blob_path, "{\"id2label\":{\"0\":\"POSITIVE\"}}").unwrap();

    let source_path = snapshots_dir.join("config.json");
    std::os::unix::fs::symlink("../../blobs/config-blob", &source_path).unwrap();

    let mut files = BTreeMap::new();
    files.insert("config.json".to_string(), source_path);
    let downloaded = DownloadedModel {
        spec: HuggingFaceModelSpec::new("owner/model", ModelTask::TextClassification)
            .name("symlink-model")
            .file("config.json"),
        files,
    };

    let bundle = ModelBundleStore::new(dir.path().join("bundles"))
        .materialize(&downloaded)
        .unwrap();
    let local_path = bundle.file_path("config.json").unwrap();
    assert_eq!(
        fs::read_to_string(&local_path).unwrap(),
        "{\"id2label\":{\"0\":\"POSITIVE\"}}"
    );
    assert!(fs::symlink_metadata(local_path)
        .unwrap()
        .file_type()
        .is_file());
}

#[test]
fn model_bundle_rejects_unsafe_remote_paths() {
    let dir = tempfile::tempdir().unwrap();
    let downloaded = fake_downloaded_model(
        &dir.path().join("cache"),
        HuggingFaceModelSpec::new("owner/model", ModelTask::TextClassification),
        &[("../config.json", "{}")],
    );

    let error = ModelBundleStore::new(dir.path().join("bundles"))
        .materialize(&downloaded)
        .unwrap_err();

    assert!(matches!(error, DetectError::InvalidArgument(_)));
}

#[test]
fn model_bundle_load_round_trips_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let downloaded = fake_downloaded_model(
        &dir.path().join("cache"),
        HuggingFaceModelSpec::new("owner/model", ModelTask::TextClassification)
            .name("round-trip")
            .revision("test-revision")
            .file("config.json"),
        &[("config.json", "{}")],
    );
    let bundle = ModelBundleStore::new(dir.path().join("bundles"))
        .materialize(&downloaded)
        .unwrap();

    let loaded = ModelBundle::load(bundle.manifest_path()).unwrap();

    assert_eq!(loaded.manifest, bundle.manifest);
    assert_eq!(
        loaded.file_path("config.json"),
        bundle.file_path("config.json")
    );
}

#[test]
fn model_bundle_store_uses_stable_safe_paths() {
    let spec = HuggingFaceModelSpec::new("owner/weird model:name", ModelTask::Custom("x".into()))
        .name("owner/weird model:name")
        .revision("refs/pr/1@abc");
    let dir = ModelBundleStore::new("bundles").bundle_dir(&spec);

    assert_eq!(
        dir,
        PathBuf::from("bundles")
            .join("owner_weird_model_name")
            .join("refs_pr_1_abc")
    );
}

#[test]
fn raw_boxes_are_clamped_and_normalized() {
    let raw = vec![RawPrediction::object(
        "person",
        0.9,
        RawBoundingBox {
            xmin: Some(-0.1),
            ymin: Some(0.1),
            xmax: Some(1.2),
            ymax: Some(0.6),
            normalized: true,
            ..RawBoundingBox::default()
        },
    )];

    let predictions = normalize_predictions(
        raw,
        &ModelTask::ObjectDetection,
        Some((100, 50)),
        PredictionRepairOptions::default(),
    );

    assert_eq!(predictions.len(), 1);
    assert_eq!(
        predictions[0].region,
        Some(BoundingBox::new(0, 5, 100, 25).unwrap())
    );
}

#[test]
fn nms_removes_overlapping_same_label_boxes() {
    let raw = vec![
        RawPrediction::object("person", 0.9, RawBoundingBox::xywh(0.0, 0.0, 10.0, 10.0)),
        RawPrediction::object("person", 0.8, RawBoundingBox::xywh(1.0, 1.0, 10.0, 10.0)),
        RawPrediction::object("car", 0.7, RawBoundingBox::xywh(1.0, 1.0, 10.0, 10.0)),
    ];

    let predictions = normalize_predictions(
        raw,
        &ModelTask::ObjectDetection,
        Some((100, 100)),
        PredictionRepairOptions::default(),
    );

    assert_eq!(predictions.len(), 2);
    assert_eq!(predictions[0].label.as_deref(), Some("person"));
    assert_eq!(predictions[1].label.as_deref(), Some("car"));
}

#[test]
fn model_video_analyzer_emits_observations() {
    struct StaticVisionBackend;

    impl VisionModelBackend for StaticVisionBackend {
        fn task(&self) -> ModelTask {
            ModelTask::ObjectDetection
        }

        fn predict_frame(&mut self, _frame: &VideoFrame<'_>) -> Result<Vec<RawPrediction>> {
            Ok(vec![RawPrediction::object(
                "car",
                0.8,
                RawBoundingBox::xywh(10.0, 10.0, 20.0, 20.0),
            )])
        }
    }

    let frame = test_frame();
    let mut analyzer = ModelVideoAnalyzer::new("objects", StaticVisionBackend);
    let observations = analyzer.process_frame(&frame.as_frame()).unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].analyzer, "objects");
    assert_eq!(observations[0].label.as_deref(), Some("car"));
    assert_eq!(observations[0].kind, ObservationKind::Object);
}

#[test]
fn model_text_analyzer_emits_dynamic_labels() {
    struct StaticTextBackend;

    impl TextModelBackend for StaticTextBackend {
        fn task(&self) -> ModelTask {
            ModelTask::TextClassification
        }

        fn predict_text(&mut self, _segment: &TextSegment<'_>) -> Result<Vec<RawPrediction>> {
            Ok(vec![RawPrediction::label("POSITIVE", 0.99)])
        }
    }

    let mut analyzer = ModelTextAnalyzer::new("sentiment", StaticTextBackend);
    let segment = TextSegment {
        segment_index: 1,
        timestamp: Some(Timestamp::new(
            30,
            video_analysis_core::Timebase::new(1, 30),
        )),
        text: "works well",
        language: Some("en"),
        is_final: true,
    };

    let events = analyzer.process_segment(&segment).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].analyzer, "sentiment");
    assert_eq!(events[0].label, "POSITIVE");
    assert_eq!(events[0].score, Some(0.99));
    assert_eq!(events[0].timestamp, segment.timestamp);
}

#[test]
fn persistent_external_command_reuses_process_for_text_predictions() {
    let model = DownloadedModel {
        spec: HuggingFaceModelSpec::new("test-model", ModelTask::TextClassification),
        files: BTreeMap::new(),
    };
    let script =
        "while IFS= read -r line; do printf '%s\\n' '{\"predictions\":[{\"label\":\"ok\",\"score\":0.5}]}'; done";
    let mut backend = PersistentExternalCommandModel::new("sh", model)
        .arg("-c")
        .arg(script);
    let segment = TextSegment {
        segment_index: 0,
        timestamp: None,
        text: "hello",
        language: Some("en"),
        is_final: true,
    };

    let first = backend.predict_text(&segment).unwrap();
    let second = backend.predict_text(&segment).unwrap();
    backend.stop().unwrap();

    assert_eq!(first[0].label.as_deref(), Some("ok"));
    assert_eq!(second[0].score, Some(0.5));
}

#[test]
fn external_command_model_returns_video_predictions() {
    let model = DownloadedModel {
        spec: HuggingFaceModelSpec::new("test-model", ModelTask::ObjectDetection),
        files: BTreeMap::new(),
    };
    let script = concat!(
        "cat >/dev/null; printf '%s' ",
        "'{\"predictions\":[{\"kind\":\"object\",\"label\":\"person\",\"score\":0.75,",
        "\"region\":{\"x\":1,\"y\":2,\"width\":3,\"height\":4}}]}'"
    );
    let mut backend = ExternalCommandModel::new("sh", model).arg("-c").arg(script);
    let frame = test_frame();

    let predictions = backend.predict_frame(&frame.as_frame()).unwrap();

    assert_eq!(predictions.len(), 1);
    assert_eq!(predictions[0].kind.as_deref(), Some("object"));
    assert_eq!(predictions[0].label.as_deref(), Some("person"));
    assert_eq!(predictions[0].score, Some(0.75));
    let region = predictions[0].region.unwrap();
    assert_eq!(region.x, Some(1.0));
    assert_eq!(region.y, Some(2.0));
    assert_eq!(region.width, Some(3.0));
    assert_eq!(region.height, Some(4.0));
}
