use std::collections::BTreeMap;

use tempfile::tempdir;

use crate::{
    DownloadedModel, ModelBundle, ModelBundleStore, ModelFileRequest, ModelRuntimeBackend,
    ModelSource, ModelSpec, ModelTask, RawPrediction,
};

#[test]
fn model_spec_records_generic_source_and_compat_fields() {
    let spec = ModelSpec::new("owner/model", ModelTask::TextEmbedding)
        .revision("v1")
        .file("config.json");

    assert_eq!(spec.repo_id_value(), Some("owner/model"));
    assert_eq!(spec.revision_value(), Some("v1"));
    assert_eq!(spec.source.kind(), "hugging_face");
    assert_eq!(spec.files, vec![ModelFileRequest::required("config.json")]);

    let local = ModelSpec::from_source(
        "local-detector",
        ModelTask::ObjectDetection,
        ModelSource::Custom("fixture".to_string()),
    );
    assert_eq!(local.repo_id_value(), None);
    assert_eq!(local.source.kind(), "fixture");
}

#[test]
fn model_bundle_store_materializes_generic_manifest() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("config.json");
    std::fs::write(&source, br#"{"model_type":"fixture"}"#).unwrap();

    let downloaded = DownloadedModel {
        spec: ModelSpec::new("owner/model", ModelTask::TextClassification)
            .name("fixture-model")
            .file("config.json"),
        files: BTreeMap::from([("config.json".to_string(), source)]),
    };

    let bundle = ModelBundleStore::new(temp.path().join("bundles"))
        .materialize(&downloaded)
        .unwrap();

    assert_eq!(bundle.manifest.name, "fixture-model");
    assert_eq!(bundle.manifest.task, ModelTask::TextClassification);
    assert!(bundle.manifest_path().exists());
    assert!(bundle.file_path("config.json").unwrap().exists());

    let loaded = ModelBundle::load(bundle.manifest_path()).unwrap();
    assert_eq!(loaded.manifest, bundle.manifest);
}

#[test]
fn blue_green_prediction_check_remains_generic() {
    let green = vec![RawPrediction::label("positive", 0.9)];
    let blue = vec![RawPrediction::label("positive", 0.90001)];

    let report = crate::compare_blue_green_predictions(
        &green,
        &blue,
        crate::BlueGreenPredictionTestOptions {
            max_score_delta: 0.001,
            compare_regions: true,
        },
    )
    .unwrap();

    assert_eq!(report.compared_predictions, 1);
    assert!(ModelRuntimeBackend::Onnx.as_str().contains("onnx"));
}

#[cfg(feature = "jobs")]
#[test]
fn model_job_spec_records_standard_metadata() {
    let spec = ModelSpec::new("owner/model", ModelTask::TextClassification).revision("v1");
    let job = crate::jobs::model_job_spec(
        "job-1",
        crate::jobs::ModelJobKind::Download,
        &spec,
        ModelRuntimeBackend::Onnx,
    )
    .unwrap();

    assert_eq!(job.kind.as_deref(), Some("model-download"));
    assert_eq!(job.metadata["model.name"], "owner/model");
    assert_eq!(job.metadata["model.task"], "text_classification");
    assert_eq!(job.metadata["model.runtime"], "onnx");
    assert_eq!(job.metadata["model.revision"], "v1");
}
