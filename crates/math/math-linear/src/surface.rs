//! Library-owned runtime surface for `math-linear`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;
use tensor_data::F32Tensor;

use crate::{F32Matrix, Kernel1d, MatrixShape};

const MAX_VALUES: usize = 100_000;

/// Describes the linear algebra operations exposed by transport wrappers.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Dense matrix and kernel contracts bridging tensor-data and vector-analysis-core.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "linear.matmul",
                "Matrix multiply",
                "Multiplies two finite f32 row-major matrices.",
                serde_json::json!({
                    "left": {"rows": 2, "cols": 2, "values": [1.0, 2.0, 3.0, 4.0]},
                    "right": {"rows": 2, "cols": 1, "values": [5.0, 6.0]}
                }),
            ),
            operation(
                "linear.transpose",
                "Matrix transpose",
                "Returns a row-major owned transpose of a finite f32 matrix.",
                serde_json::json!({
                    "matrix": {"rows": 2, "cols": 3, "values": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]}
                }),
            ),
            operation(
                "linear.solve",
                "Linear solve",
                "Solves a square finite f32 matrix against a vector or matrix right-hand side.",
                serde_json::json!({
                    "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]},
                    "rhs": [1.0, 2.0]
                }),
            ),
            operation(
                "linear.decompose",
                "LU decomposition",
                "Decomposes a square finite f32 matrix with partial-pivoted LU.",
                serde_json::json!({
                    "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]}
                }),
            ),
            operation(
                "linear.inverse",
                "Matrix inverse",
                "Returns the inverse of a finite square f32 matrix.",
                serde_json::json!({
                    "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]}
                }),
            ),
            operation(
                "linear.kernel1d",
                "1D kernel",
                "Validates and optionally normalizes a finite 1D f32 kernel.",
                serde_json::json!({"values": [0.25, 0.5, 0.25], "normalize": true}),
            ),
            operation(
                "linear.tensorBridge",
                "Tensor matrix bridge",
                "Projects rank-2 tensor payloads to matrix shape or matrix-shaped payloads to tensor shape.",
                serde_json::json!({"shape": [2, 2], "values": [1.0, 2.0, 3.0, 4.0], "direction": "tensorToMatrix"}),
            ),
        ],
    }
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    SurfaceOperation {
        id: OperationId::new(id),
        name: name.to_string(),
        description: Some(description.to_string()),
        input_schema: serde_json::json!({"type": "object", "additionalProperties": true}),
        output_schema: serde_json::json!({"type": "object"}),
        example_request,
        wasm_supported: true,
        server_supported: true,
    }
}

/// Runs one library-owned operation.
pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" => describe_value(request.input),
        "linear.matmul" => matmul_value(parse_input(request.input)?)?,
        "linear.transpose" => transpose_value(parse_input(request.input)?)?,
        "linear.solve" => solve_value(parse_input(request.input)?)?,
        "linear.decompose" => decompose_value(parse_input(request.input)?)?,
        "linear.inverse" => inverse_value(parse_input(request.input)?)?,
        "linear.kernel1d" => kernel1d_value(parse_input(request.input)?)?,
        "linear.tensorBridge" => tensor_bridge_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ));
        }
    };
    Ok(response(operation, value))
}

fn describe_value(input: serde_json::Value) -> serde_json::Value {
    let surface = package_surface();
    serde_json::json!({
        "library": surface.library,
        "version": surface.version,
        "operationCount": surface.operations.len(),
        "operations": surface
            .operations
            .iter()
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
        "input": input
    })
}

fn response(operation: OperationId, value: serde_json::Value) -> SurfaceResponse {
    SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatmulRequest {
    left: MatrixRequest,
    right: MatrixRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnaryMatrixRequest {
    matrix: MatrixRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SolveRequest {
    matrix: MatrixRequest,
    #[serde(default)]
    rhs: Option<Vec<f32>>,
    #[serde(default)]
    rhs_matrix: Option<MatrixRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixRequest {
    rows: usize,
    cols: usize,
    values: Vec<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KernelRequest {
    values: Vec<f32>,
    #[serde(default)]
    normalize: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TensorBridgeRequest {
    shape: Vec<usize>,
    values: Vec<f32>,
    direction: String,
}

fn matmul_value(request: MatmulRequest) -> Result<serde_json::Value, String> {
    let left = matrix_from_request(request.left)?;
    let right = matrix_from_request(request.right)?;
    let product = left
        .matmul(&right.as_view())
        .map_err(|error| error.to_string())?;
    matrix_json(product)
}

fn transpose_value(request: UnaryMatrixRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    let transpose = matrix
        .as_view()
        .transpose_owned()
        .map_err(|error| error.to_string())?;
    matrix_json(transpose)
}

fn solve_value(request: SolveRequest) -> Result<serde_json::Value, String> {
    if request.rhs.is_some() && request.rhs_matrix.is_some() {
        return Err("linear.solve accepts either rhs or rhsMatrix, not both".to_string());
    }
    if let Some(rhs) = request.rhs.as_ref() {
        validate_value_count(rhs.len())?;
    }
    if let Some(rhs_matrix) = request.rhs_matrix.as_ref() {
        validate_value_count(rhs_matrix.values.len())?;
    }
    let matrix = matrix_from_request(request.matrix)?;
    let decomposition = matrix
        .as_view()
        .lu_decompose()
        .map_err(|error| error.to_string())?;
    let determinant = decomposition
        .determinant()
        .map_err(|error| error.to_string())?;

    match (request.rhs, request.rhs_matrix) {
        (Some(_), Some(_)) => unreachable!("checked above"),
        (Some(rhs), None) => {
            let solution = decomposition
                .solve_vector(&rhs)
                .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "solution": solution,
                "determinant": determinant
            }))
        }
        (None, Some(rhs_matrix)) => {
            let rhs = matrix_from_request(rhs_matrix)?;
            let solution = decomposition
                .solve_matrix(&rhs.as_view())
                .map_err(|error| error.to_string())?;
            let shape = solution.shape();
            Ok(serde_json::json!({
                "solutionMatrix": {
                    "rows": shape.rows,
                    "cols": shape.cols,
                    "values": solution.values()
                },
                "determinant": determinant
            }))
        }
        (None, None) => Err("linear.solve requires rhs or rhsMatrix".to_string()),
    }
}

fn decompose_value(request: UnaryMatrixRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    let decomposition = matrix
        .as_view()
        .lu_decompose()
        .map_err(|error| error.to_string())?;
    let determinant = decomposition
        .determinant()
        .map_err(|error| error.to_string())?;
    let lower = decomposition
        .lower_matrix()
        .map_err(|error| error.to_string())?;
    let upper = decomposition
        .upper_matrix()
        .map_err(|error| error.to_string())?;
    let shape = decomposition.shape();
    Ok(serde_json::json!({
        "method": "lu",
        "rows": shape.rows,
        "cols": shape.cols,
        "pivots": decomposition.pivots(),
        "swapCount": decomposition.swap_count(),
        "determinant": determinant,
        "lower": {
            "rows": lower.shape().rows,
            "cols": lower.shape().cols,
            "values": lower.values()
        },
        "upper": {
            "rows": upper.shape().rows,
            "cols": upper.shape().cols,
            "values": upper.values()
        }
    }))
}

fn inverse_value(request: UnaryMatrixRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    let inverse = matrix.inverse().map_err(|error| error.to_string())?;
    matrix_json(inverse)
}

fn kernel1d_value(request: KernelRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    let kernel = Kernel1d::new(request.values).map_err(|error| error.to_string())?;
    let sum = kernel.values().iter().sum::<f32>();
    let mut value = serde_json::json!({
        "len": kernel.values().len(),
        "sum": sum,
        "center": kernel.values().len() / 2,
        "values": kernel.values()
    });
    if request.normalize {
        if sum.abs() <= f32::EPSILON {
            return Err("1D kernel sum must be non-zero to normalize".to_string());
        }
        value["normalizedValues"] = serde_json::json!(kernel
            .values()
            .iter()
            .map(|value| value / sum)
            .collect::<Vec<_>>());
    }
    Ok(value)
}

fn tensor_bridge_value(request: TensorBridgeRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.values.len())?;
    match request.direction.as_str() {
        "tensorToMatrix" => {
            let tensor = F32Tensor::from_dims(request.shape, request.values)
                .map_err(|error| error.to_string())?;
            let matrix = F32Matrix::try_from(&tensor).map_err(|error| error.to_string())?;
            let shape = matrix.shape();
            Ok(serde_json::json!({
                "direction": "tensorToMatrix",
                "shape": [shape.rows, shape.cols],
                "rows": shape.rows,
                "cols": shape.cols,
                "values": matrix.values()
            }))
        }
        "matrixToTensor" => {
            if request.shape.len() != 2 {
                return Err("matrixToTensor requires exactly two shape dimensions".to_string());
            }
            let matrix = F32Matrix::new(
                MatrixShape::new(request.shape[0], request.shape[1])
                    .map_err(|error| error.to_string())?,
                request.values,
            )
            .map_err(|error| error.to_string())?;
            let tensor = F32Tensor::try_from(&matrix).map_err(|error| error.to_string())?;
            Ok(serde_json::json!({
                "direction": "matrixToTensor",
                "shape": tensor.shape().dimensions(),
                "values": tensor.values()
            }))
        }
        direction => Err(format!("unsupported tensor bridge direction `{direction}`")),
    }
}

fn matrix_from_request(request: MatrixRequest) -> Result<F32Matrix, String> {
    validate_value_count(request.values.len())?;
    F32Matrix::new(
        MatrixShape::new(request.rows, request.cols).map_err(|error| error.to_string())?,
        request.values,
    )
    .map_err(|error| error.to_string())
}

fn matrix_json(matrix: F32Matrix) -> Result<serde_json::Value, String> {
    let shape = matrix.shape();
    Ok(serde_json::json!({
        "rows": shape.rows,
        "cols": shape.cols,
        "values": matrix.values()
    }))
}

fn validate_value_count(count: usize) -> Result<(), String> {
    if count > MAX_VALUES {
        return Err(format!("values must not exceed {MAX_VALUES}"));
    }
    Ok(())
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-4, "expected {left} ≈ {right}");
    }

    fn f32_array(value: &serde_json::Value) -> Vec<f32> {
        serde_json::from_value(value.clone()).expect("f32 array")
    }

    #[test]
    fn matmul_returns_product() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.matmul"),
            input: serde_json::json!({
                "left": {"rows": 2, "cols": 2, "values": [1.0, 2.0, 3.0, 4.0]},
                "right": {"rows": 2, "cols": 1, "values": [5.0, 6.0]}
            }),
        })
        .expect("matmul operation");

        assert_eq!(response.value["rows"], 2);
        assert_eq!(response.value["cols"], 1);
        assert_eq!(response.value["values"], serde_json::json!([17.0, 39.0]));
    }

    #[test]
    fn kernel_normalizes_values() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.kernel1d"),
            input: serde_json::json!({"values": [1.0, 1.0, 2.0], "normalize": true}),
        })
        .expect("kernel operation");

        assert_eq!(response.value["len"], 3);
        assert_eq!(response.value["center"], 1);
        assert_eq!(
            response.value["normalizedValues"],
            serde_json::json!([0.25, 0.25, 0.5])
        );
    }

    #[test]
    fn tensor_bridge_reports_matrix_shape() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.tensorBridge"),
            input: serde_json::json!({"shape": [2, 2], "values": [1.0, 2.0, 3.0, 4.0], "direction": "tensorToMatrix"}),
        })
        .expect("tensor bridge operation");

        assert_eq!(response.value["rows"], 2);
        assert_eq!(response.value["cols"], 2);
    }

    #[test]
    fn transpose_returns_expected_shape_and_values() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.transpose"),
            input: serde_json::json!({
                "matrix": {"rows": 2, "cols": 3, "values": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]}
            }),
        })
        .expect("transpose operation");

        assert_eq!(response.value["rows"], 3);
        assert_eq!(response.value["cols"], 2);
        assert_eq!(
            response.value["values"],
            serde_json::json!([1.0, 4.0, 2.0, 5.0, 3.0, 6.0])
        );
    }

    #[test]
    fn solve_with_vector_rhs_returns_solution() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.solve"),
            input: serde_json::json!({
                "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]},
                "rhs": [1.0, 2.0]
            }),
        })
        .expect("solve operation");

        let solution = f32_array(&response.value["solution"]);
        assert_close(solution[0], 0.2);
        assert_close(solution[1], 0.6);
        assert_close(
            serde_json::from_value(response.value["determinant"].clone()).unwrap(),
            5.0,
        );
    }

    #[test]
    fn solve_with_matrix_rhs_returns_inverse_like_result() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.solve"),
            input: serde_json::json!({
                "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]},
                "rhsMatrix": {"rows": 2, "cols": 2, "values": [1.0, 0.0, 0.0, 1.0]}
            }),
        })
        .expect("solve operation");

        assert_eq!(response.value["solutionMatrix"]["rows"], 2);
        assert_eq!(response.value["solutionMatrix"]["cols"], 2);
        let values = f32_array(&response.value["solutionMatrix"]["values"]);
        assert_close(values[0], 0.6);
        assert_close(values[1], -0.2);
        assert_close(values[2], -0.2);
        assert_close(values[3], 0.4);
    }

    #[test]
    fn decompose_returns_lower_and_upper_objects() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.decompose"),
            input: serde_json::json!({
                "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]}
            }),
        })
        .expect("decompose operation");

        assert_eq!(response.value["method"], "lu");
        assert_eq!(response.value["lower"]["rows"], 2);
        assert_eq!(response.value["upper"]["cols"], 2);
        assert!(response.value["lower"]["values"].is_array());
        assert!(response.value["upper"]["values"].is_array());
    }

    #[test]
    fn inverse_returns_expected_values() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("linear.inverse"),
            input: serde_json::json!({
                "matrix": {"rows": 2, "cols": 2, "values": [2.0, 1.0, 1.0, 3.0]}
            }),
        })
        .expect("inverse operation");

        let values = f32_array(&response.value["values"]);
        assert_close(values[0], 0.6);
        assert_close(values[1], -0.2);
        assert_close(values[2], -0.2);
        assert_close(values[3], 0.4);
    }
}
