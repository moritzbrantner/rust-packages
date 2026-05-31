#![cfg(all(feature = "native", feature = "external-tests"))]

use text_transcripts::{whisper_cpp_catalog, WhisperCppModel};

#[test]
#[ignore = "validates opt-in whisper.cpp model store and only runs when local models exist"]
fn whisper_catalog_reports_cached_models() {
    let catalog = whisper_cpp_catalog();
    assert_eq!(catalog.default_model, WhisperCppModel::default());
    assert!(!catalog.models.is_empty());
    for status in catalog.models {
        let _ = status.cached;
        let _ = status.model.file_name();
    }
}
