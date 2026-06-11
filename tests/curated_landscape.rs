use std::collections::{BTreeMap, BTreeSet};

use runtime_core::PackageSurface;

#[test]
fn declared_surfaces_expose_valid_curated_landscape_metadata() {
    let expected = expected_operations();
    let known_type_ids = runtime_core::landscape::well_known::known_type_ids()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let known_owners = runtime_core::landscape::known_owner_packages()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut function_ids = BTreeSet::new();
    let mut saw_many_cardinality = false;

    for surface in declared_surfaces() {
        let expected_operations = expected
            .get(surface.library.as_str())
            .unwrap_or_else(|| panic!("missing expected operations for {}", surface.library));
        for expected_operation in expected_operations {
            let operation = surface
                .operations
                .iter()
                .find(|operation| operation.id.as_str() == *expected_operation)
                .unwrap_or_else(|| {
                    panic!("{} missing operation {expected_operation}", surface.library)
                });
            let landscape = &operation.input_schema["xLandscape"];
            assert!(
                landscape.is_object(),
                "{} {expected_operation} missing xLandscape metadata",
                surface.library
            );
            assert_eq!(
                landscape, &operation.output_schema["xLandscape"],
                "{} {expected_operation} input/output xLandscape differ",
                surface.library
            );
            let contract: runtime_core::landscape::LandscapeOperationContract =
                serde_json::from_value(landscape.clone()).unwrap_or_else(|error| {
                    panic!(
                        "{} {expected_operation} has invalid xLandscape metadata: {error}",
                        surface.library
                    )
                });
            runtime_core::landscape::validate_landscape_contract(&contract).unwrap_or_else(
                |error| {
                    panic!(
                        "{} {expected_operation} failed validation: {error}",
                        surface.library
                    )
                },
            );
            let function = &landscape["function"];
            let function_id = function["id"].as_str().expect("function id");
            assert!(
                function_ids.insert(function_id.to_string()),
                "duplicate curated function id {function_id}"
            );
            let owner = function["owner"].as_str().expect("function owner");
            assert!(
                known_owners.contains(owner),
                "unknown curated function owner {owner}"
            );
            assert!(matches!(
                function["stability"].as_str(),
                Some("stable" | "experimental" | "internal")
            ));
            saw_many_cardinality |=
                assert_ports(&function["inputs"], &known_type_ids, &known_owners);
            saw_many_cardinality |=
                assert_ports(&function["outputs"], &known_type_ids, &known_owners);
        }
    }

    assert!(
        saw_many_cardinality,
        "expected at least one curated port with many cardinality"
    );
}

fn expected_operations() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        ("moritzbrantner-text-core", vec!["text.tokenize"]),
        (
            "moritzbrantner-text-transcripts",
            vec!["transcripts.toTextSegments"],
        ),
        (
            "moritzbrantner-image-analysis-core",
            vec!["image.core.summary"],
        ),
        ("moritzbrantner-audio-analysis-core", vec!["audio.levels"]),
        (
            "moritzbrantner-vision-core",
            vec!["vision.validateDetection", "vision.validateEmbedding"],
        ),
        (
            "moritzbrantner-vector-analysis-core",
            vec!["vector.normalize"],
        ),
        ("moritzbrantner-tensor-data", vec!["tensor.validate"]),
        ("moritzbrantner-numbers-core", vec!["numbers.summary"]),
        (
            "moritzbrantner-math-geometry-2d",
            vec!["geometry.transform"],
        ),
        ("moritzbrantner-text-analysis", vec!["analysis.document"]),
        ("moritzbrantner-text-retrieval", vec!["retrieval.search"]),
        (
            "moritzbrantner-audio-analysis-transcription",
            vec![
                "audio.transcription.transcribe",
                "audio.transcription.importWhisperX",
            ],
        ),
        (
            "moritzbrantner-image-analysis-detection",
            vec!["image.detection.colorBlob"],
        ),
        (
            "moritzbrantner-video-analysis-core",
            vec!["video.core.timecode"],
        ),
        (
            "moritzbrantner-video-analysis-detectors",
            vec!["video.detectors.compositePlan"],
        ),
        (
            "moritzbrantner-video-analysis-output",
            vec!["video.output.csvPlan"],
        ),
        (
            "moritzbrantner-video-analysis-sfm",
            vec!["video.sfm.matchPlan"],
        ),
        (
            "moritzbrantner-video-analysis-radiance-fields",
            vec!["video.radiance.cameraPath"],
        ),
        (
            "moritzbrantner-video-analysis-radiance-pipeline",
            vec!["video.radiancePipeline.assetCheck"],
        ),
    ])
}

fn declared_surfaces() -> Vec<PackageSurface> {
    vec![
        text_core::surface::package_surface(),
        text_transcripts::surface::package_surface(),
        image_analysis_core::surface::package_surface(),
        audio_analysis_core::surface::package_surface(),
        vision_core::surface::package_surface(),
        vector_analysis_core::surface::package_surface(),
        tensor_data::surface::package_surface(),
        numbers_core::surface::package_surface(),
        math_geometry_2d::surface::package_surface(),
        text_analysis::surface::package_surface(),
        text_retrieval::surface::package_surface(),
        audio_analysis_transcription::surface::package_surface(),
        image_analysis_detection::surface::package_surface(),
        video_analysis_core::surface::package_surface(),
        video_analysis_detectors::surface::package_surface(),
        video_analysis_output::surface::package_surface(),
        video_analysis_sfm::surface::package_surface(),
        video_analysis_radiance_fields::surface::package_surface(),
        video_analysis_radiance_pipeline::surface::package_surface(),
    ]
}

fn assert_ports(
    ports: &serde_json::Value,
    known_type_ids: &BTreeSet<&str>,
    known_owners: &BTreeSet<&str>,
) -> bool {
    let ports = ports.as_array().expect("ports array");
    assert!(!ports.is_empty(), "curated function must declare ports");
    let mut saw_many = false;
    for port in ports {
        let type_ref = &port["typeRef"];
        let type_id = type_ref["id"].as_str().expect("type id");
        assert!(
            known_type_ids.contains(type_id),
            "unknown type id {type_id}"
        );
        let owner = type_ref["owner"].as_str().expect("type owner");
        assert!(
            known_owners.contains(owner),
            "unknown curated type owner {owner}"
        );
        assert!(port["name"].as_str().is_some_and(|name| !name.is_empty()));
        assert!(matches!(
            port["cardinality"].as_str(),
            Some("one" | "optional" | "many")
        ));
        saw_many |= port["cardinality"].as_str() == Some("many");
    }
    saw_many
}
