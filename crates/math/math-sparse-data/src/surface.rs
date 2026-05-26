//! Library-owned runtime surface for `math-sparse-data`.

use serde::Deserialize;
use video_analysis_core::runtime::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};

use crate::{CooMatrix, CsrMatrix, SparseVector};

const MAX_VALUES: usize = 100_000;

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Describe package",
                "Sparse vector and matrix contracts for text, retrieval, and feature indexing.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "sparse.similarity",
                "Sparse similarity",
                "Computes sparse dot product or cosine similarity.",
                serde_json::json!({"left": {"dimensions": 3, "indices": [0, 2], "values": [1.0, 2.0]}, "right": {"dimensions": 3, "indices": [2], "values": [3.0]}, "metric": "dot"}),
            ),
            operation(
                "sparse.toDense",
                "Sparse to dense",
                "Converts sparse vector coordinates into a dense f32 array.",
                serde_json::json!({"dimensions": 3, "indices": [1], "values": [2.0]}),
            ),
            operation(
                "sparse.matrixSummary",
                "Sparse matrix summary",
                "Summarizes COO or CSR sparse matrix shape, nnz, density, and row nnz.",
                serde_json::json!({"format": "coo", "rows": 2, "cols": 2, "entries": [[0, 1, 2.0]]}),
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
        "sparse.similarity" => similarity_value(parse_input(request.input)?)?,
        "sparse.toDense" => to_dense_value(parse_input(request.input)?)?,
        "sparse.matrixSummary" => matrix_summary_value(parse_input(request.input)?)?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
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
        "operations": surface.operations.iter().map(|operation| operation.id.as_str()).collect::<Vec<_>>(),
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
struct SparseVectorRequest {
    dimensions: usize,
    indices: Vec<usize>,
    values: Vec<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SimilarityRequest {
    left: SparseVectorRequest,
    right: SparseVectorRequest,
    metric: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixSummaryRequest {
    format: String,
    rows: usize,
    cols: usize,
    #[serde(default)]
    entries: Vec<(usize, usize, f32)>,
    #[serde(default)]
    row_offsets: Vec<usize>,
    #[serde(default)]
    column_indices: Vec<usize>,
    #[serde(default)]
    values: Vec<f32>,
}

fn similarity_value(request: SimilarityRequest) -> Result<serde_json::Value, String> {
    let left = sparse_vector(request.left)?;
    let right = sparse_vector(request.right)?;
    let value = match request.metric.as_str() {
        "dot" => left.dot(&right).map_err(|error| error.to_string())?,
        "cosine" => left
            .cosine_similarity(&right)
            .map_err(|error| error.to_string())?,
        metric => return Err(format!("unsupported sparse similarity metric `{metric}`")),
    };
    Ok(serde_json::json!({"metric": request.metric, "value": value}))
}

fn to_dense_value(request: SparseVectorRequest) -> Result<serde_json::Value, String> {
    let vector = sparse_vector(request)?;
    Ok(serde_json::json!({
        "dimensions": vector.dimensions(),
        "nnz": vector.nnz(),
        "dense": vector.to_dense()
    }))
}

fn matrix_summary_value(request: MatrixSummaryRequest) -> Result<serde_json::Value, String> {
    match request.format.as_str() {
        "coo" => {
            validate_value_count(request.entries.len())?;
            let coo = CooMatrix::new(request.rows, request.cols, request.entries)
                .map_err(|error| error.to_string())?;
            let csr = coo.to_csr().map_err(|error| error.to_string())?;
            matrix_json("coo", csr)
        }
        "csr" => {
            validate_value_count(request.values.len())?;
            let csr = CsrMatrix::new(
                request.rows,
                request.cols,
                request.row_offsets,
                request.column_indices,
                request.values,
            )
            .map_err(|error| error.to_string())?;
            matrix_json("csr", csr)
        }
        format => Err(format!("unsupported sparse matrix format `{format}`")),
    }
}

fn matrix_json(format: &str, matrix: CsrMatrix) -> Result<serde_json::Value, String> {
    let row_nnz = matrix
        .rows_iter()
        .map(|row| row.indices().len())
        .collect::<Vec<_>>();
    let nnz = row_nnz.iter().sum::<usize>();
    Ok(serde_json::json!({
        "format": format,
        "rows": matrix.rows(),
        "cols": matrix.cols(),
        "nnz": nnz,
        "density": nnz as f64 / (matrix.rows() * matrix.cols()) as f64,
        "rowNnz": row_nnz
    }))
}

fn sparse_vector(request: SparseVectorRequest) -> Result<SparseVector, String> {
    validate_value_count(request.values.len())?;
    SparseVector::new(request.dimensions, request.indices, request.values)
        .map_err(|error| error.to_string())
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

    #[test]
    fn sparse_similarity_dot_works() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("sparse.similarity"),
            input: serde_json::json!({"left": {"dimensions": 3, "indices": [0, 2], "values": [1.0, 2.0]}, "right": {"dimensions": 3, "indices": [2], "values": [3.0]}, "metric": "dot"}),
        }).expect("similarity");
        assert_eq!(response.value["value"], 6.0);
    }

    #[test]
    fn sparse_to_dense_works() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("sparse.toDense"),
            input: serde_json::json!({"dimensions": 3, "indices": [1], "values": [2.0]}),
        })
        .expect("to dense");
        assert_eq!(response.value["dense"], serde_json::json!([0.0, 2.0, 0.0]));
    }

    #[test]
    fn sparse_matrix_summary_reports_row_counts() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("sparse.matrixSummary"),
            input: serde_json::json!({"format": "coo", "rows": 2, "cols": 3, "entries": [[0, 1, 2.0], [1, 2, 3.0]]}),
        }).expect("matrix summary");
        assert_eq!(response.value["nnz"], 2);
        assert_eq!(response.value["rowNnz"], serde_json::json!([1, 1]));
    }
}
