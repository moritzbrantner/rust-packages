#![cfg(feature = "external-tests")]

use std::path::PathBuf;

use model_runtime::{resolve_or_download_bundle, ModelBundleResolveOptions, ModelSpec, ModelTask};
use runtime_onnx::{
    OnnxDimension, OnnxGraphOptimization, OnnxIoInfo, OnnxNamedTensor, OnnxRunner, OnnxSession,
    OnnxSessionOptions, OnnxTensor, OnnxTensorElementType, OnnxTensorValue,
};
use tempfile::tempdir;

#[test]
#[ignore = "downloads a public Hugging Face ONNX fixture and requires ONNX Runtime"]
fn model_runtime_spine_downloads_bundle_and_runs_tiny_onnx(
) -> Result<(), Box<dyn std::error::Error>> {
    configure_onnx_runtime_dylib()?;

    let spec = ModelSpec::new(
        "onnx-internal-testing/tiny-random-BertModel-ONNX",
        ModelTask::Custom("onnx_spine_smoke".to_string()),
    )
    .name("tiny-random-bert-onnx-spine-smoke")
    .file("onnx/model.onnx");

    let temp = tempdir()?;
    let options = ModelBundleResolveOptions {
        bundle_root: temp.path().join("bundles"),
        download_progress: false,
        max_retries: 1,
        ..ModelBundleResolveOptions::default()
    };

    let bundle = resolve_or_download_bundle(&spec, &options)?;
    let model_path = bundle
        .file_path("onnx/model.onnx")
        .ok_or("materialized bundle did not contain onnx/model.onnx")?;
    assert!(model_path.is_file(), "missing {}", model_path.display());
    eprintln!("materialized ONNX smoke model at {}", model_path.display());

    let mut session = OnnxSession::from_file_with_options(
        &model_path,
        OnnxSessionOptions {
            graph_optimization: OnnxGraphOptimization::Disable,
            ..OnnxSessionOptions::default()
        },
    )?;
    eprintln!("opened ONNX Runtime session");
    let metadata = session.metadata()?;
    eprintln!(
        "loaded metadata for {} input(s) and {} output(s)",
        metadata.inputs.len(),
        metadata.outputs.len()
    );
    assert!(
        !metadata.inputs.is_empty(),
        "ONNX smoke model should expose at least one input"
    );
    assert!(
        !metadata.outputs.is_empty(),
        "ONNX smoke model should expose at least one output"
    );

    let inputs = metadata
        .inputs
        .iter()
        .map(dummy_input)
        .collect::<Result<Vec<_>, _>>()?;
    eprintln!("built {} dummy input tensor(s)", inputs.len());
    let outputs = session.run(inputs)?;
    eprintln!("received {} output tensor(s)", outputs.len());
    assert!(
        !outputs.is_empty(),
        "ONNX smoke model should produce at least one output"
    );

    Ok(())
}

fn dummy_input(info: &OnnxIoInfo) -> Result<OnnxNamedTensor, Box<dyn std::error::Error>> {
    let shape = concrete_shape(info);
    let element_count = shape.iter().try_fold(1usize, |acc, dim| {
        acc.checked_mul(*dim)
            .ok_or_else(|| format!("input `{}` shape {:?} overflows", info.name, shape))
    })?;
    let element_type = info
        .element_type
        .ok_or_else(|| format!("input `{}` is not a supported tensor", info.name))?;

    let tensor = match element_type {
        OnnxTensorElementType::F32 => {
            OnnxTensorValue::F32(OnnxTensor::new(shape, vec![0.0; element_count])?)
        }
        OnnxTensorElementType::I64 => {
            OnnxTensorValue::I64(OnnxTensor::new(shape, vec![0_i64; element_count])?)
        }
        OnnxTensorElementType::I32 => {
            OnnxTensorValue::I32(OnnxTensor::new(shape, vec![0_i32; element_count])?)
        }
        OnnxTensorElementType::U8 => {
            OnnxTensorValue::U8(OnnxTensor::new(shape, vec![0_u8; element_count])?)
        }
    };

    Ok(OnnxNamedTensor {
        name: info.name.clone(),
        tensor,
    })
}

fn concrete_shape(info: &OnnxIoInfo) -> Vec<usize> {
    info.dimensions
        .iter()
        .map(|dimension| match dimension {
            OnnxDimension::Fixed(value) => *value,
            OnnxDimension::Symbolic(_) | OnnxDimension::Unknown => 1,
        })
        .collect()
}

fn configure_onnx_runtime_dylib() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return Ok(());
    }
    let Some(path) = local_onnxruntime_dylib()? else {
        return Err(
            "ORT_DYLIB_PATH is not set and no local ONNX Runtime library was found; run `bash scripts/setup_model_external_tools.sh onnx` or set ORT_DYLIB_PATH"
                .into(),
        );
    };
    std::env::set_var("ORT_DYLIB_PATH", &path);
    eprintln!("using ONNX Runtime dylib at {}", path.display());
    Ok(())
}

fn local_onnxruntime_dylib() -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tools_dir = std::env::var_os("MODEL_TOOLS_DIR")
        .or_else(|| std::env::var_os("EXTERNAL_TEST_TOOLS_DIR"))
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".external-test-tools"));
    let python_lib_dir = tools_dir.join("model-python-venv").join("lib");
    if !python_lib_dir.is_dir() {
        return Ok(None);
    }

    for python_dir in std::fs::read_dir(python_lib_dir)? {
        let python_dir = python_dir?.path();
        let Some(name) = python_dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("python") {
            continue;
        }
        let capi_dir = python_dir
            .join("site-packages")
            .join("onnxruntime")
            .join("capi");
        if !capi_dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(capi_dir)? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if matches!(name, "onnxruntime.dll" | "libonnxruntime.dylib")
                || name.starts_with("libonnxruntime.so")
            {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}
