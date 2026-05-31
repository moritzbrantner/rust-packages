use std::io::Write;

use osmpbfreader::{fileformat, osmformat};
use protobuf::Message;

#[test]
fn cli_surface_delegates_to_library() {
    assert_eq!(geo_io_osm_cli::LIBRARY_CRATE, "geo-io-osm");
    let surface = geo_io_osm_cli::package_surface();
    assert_eq!(surface.library, "moritzbrantner-geo-io-osm");
    assert!(surface
        .operations
        .iter()
        .any(|operation| operation.id.as_str() == "osm.filterPbfBase64"));
}

#[test]
fn cli_run_validates_spec() {
    let response = geo_io_osm_cli::run_operation(
        "osm.validateSpec",
        serde_json::json!({"spec": {"filter": {"types": ["node"]}}}),
    )
    .unwrap();
    assert_eq!(response.value["result"]["valid"], true);
    assert_eq!(response.value["operation"], "osm.validateSpec");
}

#[test]
fn cli_filter_writes_geojson_from_pbf_and_spec() {
    let temp = tempfile::tempdir().expect("tempdir");
    let input = temp.path().join("sample.osm.pbf");
    let spec = temp.path().join("spec.json");
    let output = temp.path().join("out.geojson");
    std::fs::write(&input, synthetic_pbf_bytes()).expect("write pbf");
    std::fs::write(
        &spec,
        r#"{"filter":{"types":["node"],"include":{"all":[{"key":"amenity","value":"school"}]}}}"#,
    )
    .expect("write spec");

    let feature_count =
        geo_io_osm_cli::filter_to_geojson(&input, &spec, &output).expect("filter to GeoJSON");
    let geojson: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&output).expect("read output"))
            .expect("parse GeoJSON output");

    assert_eq!(feature_count, 1);
    assert_eq!(geojson["features"][0]["id"], "node/1");
    assert_eq!(geojson["features"][0]["properties"]["amenity"], "school");
}

fn synthetic_pbf_bytes() -> Vec<u8> {
    let mut string_table = osmformat::StringTable::new();
    for value in ["", "amenity", "school"] {
        string_table.mut_s().push(value.as_bytes().to_vec());
    }

    let mut dense_nodes = osmformat::DenseNodes::new();
    dense_nodes.id = vec![1];
    dense_nodes.lat = vec![480_000_000];
    dense_nodes.lon = vec![80_000_000];
    dense_nodes.keys_vals = vec![1, 2, 0];

    let mut group = osmformat::PrimitiveGroup::new();
    group.set_dense(dense_nodes);

    let mut block = osmformat::PrimitiveBlock::new();
    block.set_stringtable(string_table);
    block.mut_primitivegroup().push(group);

    let mut bytes = Vec::new();
    write_raw_blob(&mut bytes, "OSMData", block.write_to_bytes().unwrap());
    bytes
}

fn write_raw_blob(writer: &mut Vec<u8>, field_type: &str, payload: Vec<u8>) {
    let mut blob = fileformat::Blob::new();
    blob.set_raw(payload);
    let blob_bytes = blob.write_to_bytes().unwrap();

    let mut header = fileformat::BlobHeader::new();
    header.set_field_type(field_type.to_owned());
    header.set_datasize(blob_bytes.len().try_into().unwrap());
    let header_bytes = header.write_to_bytes().unwrap();

    let header_len: u32 = header_bytes.len().try_into().unwrap();
    writer.write_all(&header_len.to_be_bytes()).unwrap();
    writer.write_all(&header_bytes).unwrap();
    writer.write_all(&blob_bytes).unwrap();
}
