use tempfile::tempdir;

use image_analysis_classification::{image_classification_catalog, ImageClassificationTask};
use image_analysis_core::{ImagePixelFormat, ImageView, OwnedImage};
use image_analysis_detection::{FaceBox, FaceDetectionPreset};
use image_analysis_embeddings::{
    image_embedding_catalog, FaceEmbeddingPreset, ImageEmbeddingPreset, ImageEmbeddingTask,
};
use image_analysis_io::{read_image, write_image};
use image_analysis_ocr::{
    OcrDocument, OcrPreset, OcrRequest, OcrTechnique, OcrTextContractOptions,
};
use image_analysis_segmentation::ImageSegmentationRequest;

#[test]
fn image_public_api_covers_core_defaults_and_io() -> Result<(), Box<dyn std::error::Error>> {
    let rgb = OwnedImage::new_rgb(2, 1, vec![255, 0, 0, 0, 255, 0])?;
    let gray = OwnedImage::new_gray(2, 1, vec![0, 255])?;
    let view = ImageView::packed(2, 1, ImagePixelFormat::Gray8, &[0, 255])?;
    assert_eq!(rgb.pixel_format, ImagePixelFormat::Rgb24);
    assert_eq!(gray.pixel_format, ImagePixelFormat::Gray8);
    assert_eq!(view.stride, 2);

    let default_request = ImageSegmentationRequest::default();
    assert!(!default_request.prompt.automatic_mask_generation);
    let automatic = ImageSegmentationRequest::automatic_mask_generation();
    assert!(automatic.prompt.automatic_mask_generation);

    let temp = tempdir()?;
    let path = temp.path().join("roundtrip.png");
    write_image(&path, &rgb)?;
    let roundtrip = read_image(&path)?;
    assert_eq!(roundtrip, rgb);

    Ok(())
}

#[test]
fn image_public_api_covers_ocr_model_presets_and_requests() {
    let request = OcrRequest::new()
        .model_preset(OcrPreset::TrOcrBaseHandwritten)
        .languages(["en"])
        .preserve_layout(true);

    match request.technique {
        Some(OcrTechnique::HuggingFaceModel(spec)) => {
            assert_eq!(
                spec.repo_id_value(),
                Some("microsoft/trocr-base-handwritten")
            );
        }
        _ => panic!("expected a Hugging Face OCR model technique"),
    }
    assert_eq!(request.languages, ["en".to_string()]);

    let onnx_request = OcrRequest::new().model_preset(OcrPreset::TrOcrBasePrintedOnnx);
    match onnx_request.technique {
        Some(OcrTechnique::OnnxModel(spec)) => {
            assert_eq!(spec.repo_id_value(), Some("Xenova/trocr-base-printed"));
        }
        _ => panic!("expected an ONNX OCR model technique"),
    }

    let conversion = OcrDocument::new("Hello OCR", 8, 4)
        .unwrap()
        .to_text_document_contract(OcrTextContractOptions {
            id: "ocr-public-api".to_string(),
            ..OcrTextContractOptions::default()
        });
    assert_eq!(conversion.document.id, "ocr-public-api");
    assert_eq!(
        conversion
            .document
            .source
            .and_then(|source| source.source_kind),
        Some("ocr_image".to_string())
    );
}

#[test]
fn image_public_api_covers_embedding_and_face_model_presets() {
    let clip = ImageEmbeddingPreset::XenovaClipVitBasePatch32Onnx.model_spec();
    assert_eq!(clip.repo_id_value(), Some("Xenova/clip-vit-base-patch32"));

    let detector = FaceDetectionPreset::OpenCvYuNet.model_spec();
    assert_eq!(
        detector.repo_id_value(),
        Some("opencv/face_detection_yunet")
    );

    let embedder = FaceEmbeddingPreset::OpenCvSFace.model_spec();
    assert_eq!(
        embedder.repo_id_value(),
        Some("opencv/face_recognition_sface")
    );

    assert!(FaceBox::new(0.1, 0.2, 0.3, 0.4).is_ok());
}

#[test]
fn image_public_api_covers_classification_catalog_compatibility() {
    let classifications =
        image_classification_catalog(Some(ImageClassificationTask::ImageClassification));
    assert!(classifications
        .iter()
        .any(|entry| entry.id == "vit-base-patch16-224"));

    let catalog = image_embedding_catalog(Some(ImageEmbeddingTask::ImageEmbedding));
    assert!(catalog.iter().any(|entry| {
        entry.id == "xenova-clip-vit-base-patch32-onnx"
            && entry.task == ImageEmbeddingTask::ImageEmbedding
    }));
}
