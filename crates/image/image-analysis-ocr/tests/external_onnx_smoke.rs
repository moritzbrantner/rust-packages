#![cfg(feature = "external-tests")]

use std::path::{Path, PathBuf};

use image_analysis_core::OwnedImage;
use image_analysis_ocr::{OcrBackend, OcrRequest, OnnxTrOcrBackend};
use model_runtime::ModelBundle;

fn bundle(names: &[&str]) -> Option<ModelBundle> {
    let mut roots = Vec::new();
    for name in names {
        let root = PathBuf::from(".model-runtime").join(name).join("main");
        if root.join("manifest.json").is_file() {
            return Some(ModelBundle::load(Path::new(&root)).expect("load model bundle"));
        }
        roots.push(root);
    }
    eprintln!(
        "skipping external ONNX smoke test; missing any manifest at {}",
        roots
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    None
}

#[test]
#[ignore = "requires local TrOCR ONNX bundle in .model-runtime"]
fn trocr_onnx_returns_non_empty_ocr_document() {
    let Some(bundle) = bundle(&["trocr-base-printed-onnx", "trocr-base-printed"]) else {
        return;
    };
    let image = OwnedImage::new_rgb(16, 16, vec![255; 16 * 16 * 3]).unwrap();
    let mut backend = OnnxTrOcrBackend::from_bundle(bundle).unwrap();
    let document = backend
        .recognize_image(&image.as_view(), &OcrRequest::new().languages(["en"]))
        .unwrap();
    assert!(!document.text.trim().is_empty());
    assert_eq!((document.width, document.height), (16, 16));
}
