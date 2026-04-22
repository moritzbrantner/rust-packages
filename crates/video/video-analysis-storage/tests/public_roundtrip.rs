use video_analysis_storage::{
    build_manifest, read_dataset_dir, read_dataset_json, read_jsonl, write_dataset_dir,
    write_dataset_json, write_jsonl,
};

#[test]
fn dataset_json_jsonl_and_manifest_round_trip_public_records() {
    let dataset = video_analysis_test_support::dataset_with_scene_text_and_feature();
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("dataset.json");
    let jsonl_path = dir.path().join("records.jsonl");

    write_dataset_json(&json_path, &dataset).unwrap();
    write_jsonl(&jsonl_path, &dataset).unwrap();

    let from_json = read_dataset_json(&json_path).unwrap();
    let from_jsonl = read_jsonl(&jsonl_path).unwrap();
    assert_eq!(from_json.records, dataset.records);
    assert_eq!(from_jsonl.records, dataset.records);

    let manifest = build_manifest(&dataset, "records.jsonl");
    assert_eq!(manifest.record_count, dataset.records.len() as u64);
    assert_eq!(manifest.record_counts["scene"], 1);
    assert_eq!(manifest.record_counts["feature"], 1);

    let dataset_dir = dir.path().join("dataset-dir");
    let written_manifest = write_dataset_dir(&dataset_dir, &dataset).unwrap();
    assert_eq!(written_manifest.record_count, manifest.record_count);
    let from_dir = read_dataset_dir(&dataset_dir).unwrap();
    assert_eq!(from_dir.records, dataset.records);
}
