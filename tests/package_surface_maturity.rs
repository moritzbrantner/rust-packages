use std::collections::BTreeSet;

use video_analysis_core::runtime::PackageSurface;

#[test]
fn included_library_crates_expose_mature_runtime_surfaces() {
    for surface in included_surfaces() {
        assert!(
            surface
                .operations
                .iter()
                .any(|operation| operation.id.as_str() == "describe"),
            "{} must expose describe",
            surface.library
        );
        assert!(
            surface.operations.len() >= 3,
            "{} must expose describe plus at least two crate-specific operations",
            surface.library
        );

        let mut ids = BTreeSet::new();
        for operation in &surface.operations {
            assert!(
                ids.insert(operation.id.as_str()),
                "{} has duplicate operation id {}",
                surface.library,
                operation.id.as_str()
            );
        }
    }
}

#[test]
fn included_image_task_crates_are_not_describe_only() {
    for surface in image_task_surfaces() {
        assert!(
            surface
                .operations
                .iter()
                .any(|operation| operation.id.as_str() != "describe"),
            "{} must expose deterministic image task operations",
            surface.library
        );
    }
}

#[test]
fn every_text_crate_has_at_least_three_operations_including_describe() {
    for surface in text_surfaces() {
        assert!(
            surface.operations.len() >= 3,
            "{} must expose at least three operations",
            surface.library
        );
        assert!(
            surface
                .operations
                .iter()
                .any(|operation| operation.id.as_str() == "describe"),
            "{} must expose describe",
            surface.library
        );
    }
}

fn included_surfaces() -> Vec<PackageSurface> {
    let mut surfaces = Vec::new();
    surfaces.extend(text_surfaces());
    surfaces.extend(image_task_surfaces());
    surfaces.extend([
        image_analysis_core::surface::package_surface(),
        image_analysis_processing::surface::package_surface(),
        data_inversion_core::surface::package_surface(),
        dense_data::surface::package_surface(),
        geo_data::surface::package_surface(),
        graph_analysis_core::surface::package_surface(),
        numbers_core::surface::package_surface(),
        tensor_data::surface::package_surface(),
        finance_statistics::surface::package_surface(),
        maps_kernels_core::surface::package_surface(),
        math_geometry_2d::surface::package_surface(),
        math_linear::surface::package_surface(),
        math_signal_core::surface::package_surface(),
        math_sparse_data::surface::package_surface(),
        math_statistics::surface::package_surface(),
        vector_analysis_core::surface::package_surface(),
        vector_analysis_index::surface::package_surface(),
        jobs_core::surface::package_surface(),
        model_runtime::surface::package_surface(),
        video_analysis_test_support::surface::package_surface(),
    ]);
    surfaces
}

fn text_surfaces() -> Vec<PackageSurface> {
    vec![
        text_core::surface::package_surface(),
        text_analysis::surface::package_surface(),
        text_classification::surface::package_surface(),
        text_embeddings::surface::package_surface(),
        text_generation::surface::package_surface(),
        text_generation_linguistics::surface::package_surface(),
        text_lexical::surface::package_surface(),
        text_linguistics::surface::package_surface(),
        text_model_runtime::surface::package_surface(),
        text_question_answering::surface::package_surface(),
        text_retrieval::surface::package_surface(),
        text_transcripts::surface::package_surface(),
    ]
}

fn image_task_surfaces() -> Vec<PackageSurface> {
    vec![
        image_analysis_captioning::surface::package_surface(),
        image_analysis_classification::surface::package_surface(),
        image_analysis_detection::surface::package_surface(),
        image_analysis_embeddings::surface::package_surface(),
        image_analysis_io::surface::package_surface(),
        image_analysis_ocr::surface::package_surface(),
        image_analysis_segmentation::surface::package_surface(),
        image_analysis_synthesis::surface::package_surface(),
    ]
}
