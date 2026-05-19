use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{tempdir, TempDir};
use three_d_processing_core::Point3;
use three_d_processing_io::{
    read_gltf_mesh, read_mesh, read_obj_mesh, read_ply_mesh, write_gltf_mesh, write_mesh,
    write_obj_mesh, write_ply_mesh,
};
use three_d_processing_mesh::{Mesh, Triangle};
use video_analysis_core::DetectError;

const MINIMAL_OBJ: &str = include_str!("fixtures/minimal.obj");
const MINIMAL_PLY: &str = include_str!("fixtures/minimal.ply");
const MINIMAL_GLTF: &str = include_str!("fixtures/minimal.gltf");

fn minimal_mesh() -> Mesh {
    Mesh::new(
        [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        [Triangle::new(0, 1, 2)],
    )
    .unwrap()
}

fn fixture_path(dir: &TempDir, name: &str, contents: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, contents).unwrap();
    path
}

fn assert_invalid_argument<T>(result: video_analysis_core::Result<T>, expected: &str) {
    match result {
        Err(DetectError::InvalidArgument(message)) => {
            assert!(
                message.contains(expected),
                "expected `{message}` to contain `{expected}`"
            );
        }
        Err(error) => panic!("expected invalid argument containing `{expected}`, got {error}"),
        Ok(_) => panic!("expected invalid argument containing `{expected}`"),
    }
}

#[test]
fn reads_minimal_obj_fixture() {
    let dir = tempdir().unwrap();
    let path = fixture_path(&dir, "minimal.obj", MINIMAL_OBJ);

    assert_eq!(read_obj_mesh(path).unwrap(), minimal_mesh());
}

#[test]
fn reads_minimal_ply_fixture() {
    let dir = tempdir().unwrap();
    let path = fixture_path(&dir, "minimal.ply", MINIMAL_PLY);

    assert_eq!(read_ply_mesh(path).unwrap(), minimal_mesh());
}

#[test]
fn reads_embedded_gltf_fixture() {
    let dir = tempdir().unwrap();
    let path = fixture_path(&dir, "minimal.gltf", MINIMAL_GLTF);

    assert_eq!(read_gltf_mesh(path).unwrap(), minimal_mesh());
}

#[test]
fn simple_mesh_api_round_trips_all_supported_formats() {
    let dir = tempdir().unwrap();
    let mesh = minimal_mesh();

    let obj = dir.path().join("mesh.obj");
    write_mesh(&obj, &mesh).unwrap();
    assert_eq!(read_mesh(&obj).unwrap(), mesh);

    let ply = dir.path().join("mesh.ply");
    write_mesh(&ply, &mesh).unwrap();
    assert_eq!(read_mesh(&ply).unwrap(), mesh);

    let gltf = dir.path().join("mesh.gltf");
    write_mesh(&gltf, &mesh).unwrap();
    assert_eq!(read_mesh(&gltf).unwrap(), mesh);
}

#[test]
fn simple_format_writers_preserve_current_minimal_contract() {
    let dir = tempdir().unwrap();
    let mesh = minimal_mesh();

    let obj = dir.path().join("mesh.obj");
    write_obj_mesh(&obj, &mesh).unwrap();
    assert_eq!(read_obj_mesh(&obj).unwrap(), mesh);

    let ply = dir.path().join("mesh.ply");
    write_ply_mesh(&ply, &mesh).unwrap();
    assert_eq!(read_ply_mesh(&ply).unwrap(), mesh);

    let gltf = dir.path().join("mesh.gltf");
    write_gltf_mesh(&gltf, &mesh).unwrap();
    assert_eq!(read_gltf_mesh(&gltf).unwrap(), mesh);
}

#[test]
fn rejects_unsupported_mesh_extensions() {
    assert_invalid_argument(
        read_mesh(Path::new("mesh.stl")),
        "unsupported mesh extension",
    );
    assert_invalid_argument(
        write_mesh(Path::new("mesh.stl"), &minimal_mesh()),
        "unsupported mesh extension",
    );
}

#[test]
fn rejects_invalid_obj_indices() {
    let dir = tempdir().unwrap();

    let zero = fixture_path(&dir, "zero.obj", "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 0 2 3\n");
    assert_invalid_argument(
        read_obj_mesh(zero),
        "OBJ indices must be 1-based and non-zero",
    );

    let out_of_bounds = fixture_path(
        &dir,
        "out-of-bounds.obj",
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 4\n",
    );
    assert_invalid_argument(
        read_obj_mesh(out_of_bounds),
        "triangle vertex index is out of bounds",
    );
}

#[test]
fn rejects_broken_obj_face_syntax() {
    let dir = tempdir().unwrap();
    let path = fixture_path(&dir, "broken.obj", "v 0 0 0\nv 1 0 0\nf 1 two 3\n");

    assert_invalid_argument(read_obj_mesh(path), "invalid OBJ face index");
}

#[test]
fn rejects_non_finite_obj_vertices() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "non-finite.obj",
        "v 0 0 0\nv inf 0 0\nv 0 1 0\nf 1 2 3\n",
    );

    assert_invalid_argument(read_obj_mesh(path), "mesh vertices must be finite");
}

#[test]
fn rejects_bad_ply_headers() {
    let dir = tempdir().unwrap();

    let wrong_magic = fixture_path(&dir, "wrong-magic.ply", "not-ply\n");
    assert_invalid_argument(read_ply_mesh(wrong_magic), "PLY file must start with `ply`");

    let missing_vertex_count = fixture_path(
        &dir,
        "missing-vertex-count.ply",
        "ply\nformat ascii 1.0\nend_header\n",
    );
    assert_invalid_argument(
        read_ply_mesh(missing_vertex_count),
        "PLY header must declare vertex count",
    );
}

#[test]
fn rejects_invalid_ply_indices() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "out-of-bounds.ply",
        "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n0 1 0\n3 0 1 3\n",
    );

    assert_invalid_argument(
        read_ply_mesh(path),
        "triangle vertex index is out of bounds",
    );
}

#[test]
fn rejects_non_finite_ply_vertices() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "non-finite.ply",
        "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\nNaN 0 0\n0 1 0\n3 0 1 2\n",
    );

    assert_invalid_argument(read_ply_mesh(path), "mesh vertices must be finite");
}

#[test]
fn rejects_truncated_ply_bodies() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "truncated.ply",
        "ply\nformat ascii 1.0\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n",
    );

    assert_invalid_argument(read_ply_mesh(path), "unexpected end of file");
}

#[test]
fn rejects_truncated_gltf_position_buffer() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "truncated-position.gltf",
        &MINIMAL_GLTF
            .replace("\"byteLength\": 48", "\"byteLength\": 4")
            .replace(
                "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAEAAAACAAAA",
                "AAAAAA==",
            ),
    );

    assert_invalid_argument(
        read_gltf_mesh(path),
        "glTF POSITION accessor exceeds buffer length",
    );
}

#[test]
fn rejects_truncated_gltf_index_buffer() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "truncated-index.gltf",
        &MINIMAL_GLTF
            .replace("\"byteLength\": 48", "\"byteLength\": 40")
            .replace(
                "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAEAAAACAAAA",
                "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAA==",
            ),
    );

    assert_invalid_argument(
        read_gltf_mesh(path),
        "glTF index accessor exceeds buffer length",
    );
}

#[test]
fn rejects_invalid_gltf_indices() {
    let dir = tempdir().unwrap();
    let path = fixture_path(
        &dir,
        "bad-index.gltf",
        &MINIMAL_GLTF.replace(
            "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAEAAAACAAAA",
            "AAAAAAAAAAAAAAAAAACAPwAAAAAAAAAAAAAAAAAAgD8AAAAAAAAAAAQAAAACAAAA",
        ),
    );

    assert_invalid_argument(
        read_gltf_mesh(path),
        "triangle vertex index is out of bounds",
    );
}
