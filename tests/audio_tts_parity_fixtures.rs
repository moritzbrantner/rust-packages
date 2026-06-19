const F5_E2_PARITY_FIXTURES: &[u8] = include_bytes!("fixtures/f5-e2-tts-parity-fixtures.json");

#[test]
fn f5_e2_parity_fixture_metadata_documents_reference_gaps() {
    let fixture: serde_json::Value =
        serde_json::from_slice(F5_E2_PARITY_FIXTURES).expect("parity fixture json");
    assert_eq!(fixture["schemaVersion"], 1);
    assert_eq!(
        fixture["scope"],
        "audio-generation-tts-python-reference-parity"
    );

    let upstream = fixture["upstreamReferences"]
        .as_array()
        .expect("upstream references");
    let upstream_model_ids = upstream
        .iter()
        .map(|reference| reference["modelId"].as_str().expect("model id"))
        .collect::<Vec<_>>();
    assert!(upstream_model_ids.contains(&"f5-tts-v1-base"));
    assert!(upstream_model_ids.contains(&"f5-tts-base"));
    assert!(upstream_model_ids.contains(&"e2-tts-base"));

    let fixtures = fixture["fixtures"].as_array().expect("fixtures");
    assert!(fixtures
        .iter()
        .any(|fixture| fixture["id"] == "f5-long-text-chunking"
            && fixture["expectedNativeBehavior"]["textChunkingStrategy"] == "sentence-boundary"));
    assert!(fixtures
        .iter()
        .any(|fixture| fixture["id"] == "e2-reference-metadata"
            && fixture["expectedNativeBehavior"]["status"] == "unsupportedRuntime"));

    for fixture in fixtures {
        assert!(
            fixture["knownGaps"]
                .as_array()
                .is_some_and(|gaps| !gaps.is_empty()),
            "{} must document known gaps",
            fixture["id"]
        );
    }
}
