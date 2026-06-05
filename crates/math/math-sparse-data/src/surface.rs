//! Library-owned runtime surface for `math-sparse-data`.

use runtime_core::{
    OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation, SurfaceRequest,
    SurfaceResponse,
};
use serde::Deserialize;

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
            operation(
                "sparse.matrixStats",
                "Sparse matrix stats",
                "Summarizes sparse matrix density, row/column nnz, row/column sums, and compact nnz statistics.",
                serde_json::json!({"matrix": {"rows": 3, "cols": 4, "entries": [[0, 1, 2.0], [1, 3, 4.0], [2, 1, -1.0]]}}),
            ),
            operation(
                "sparse.vectorOps",
                "Sparse vector operations",
                "Computes sparse vector norms, optional scaling, optional addition, and top-k entries.",
                serde_json::json!({"vector": {"dimensions": 4, "indices": [0, 2], "values": [1.0, -3.0]}, "scale": 0.5, "topK": 1}),
            ),
            operation(
                "sparse.matrixVector",
                "Sparse matrix vector multiply",
                "Multiplies a COO or CSR sparse matrix by a finite dense vector.",
                serde_json::json!({"format": "coo", "rows": 2, "cols": 3, "entries": [[0, 1, 2.0], [1, 2, 3.0]], "vector": [1.0, 2.0, 3.0]}),
            ),
            operation(
                "sparse.transpose",
                "Sparse transpose",
                "Transposes a COO or CSR sparse matrix and returns canonical COO entries.",
                serde_json::json!({"format": "coo", "rows": 2, "cols": 3, "entries": [[0, 1, 2.0], [1, 2, 3.0]]}),
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
        "sparse.matrixStats" => matrix_stats_value(parse_input(request.input)?)?,
        "sparse.vectorOps" => vector_ops_value(parse_input(request.input)?)?,
        "sparse.matrixVector" => matrix_vector_value(parse_input(request.input)?)?,
        "sparse.transpose" => transpose_value(parse_input(request.input)?)?,
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
    #[serde(default = "default_sparse_format")]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixStatsRequest {
    matrix: MatrixSummaryRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VectorOpsRequest {
    vector: SparseVectorRequest,
    #[serde(default)]
    scale: Option<f32>,
    #[serde(default)]
    add: Option<SparseVectorRequest>,
    #[serde(default)]
    top_k: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixVectorRequest {
    #[serde(flatten)]
    matrix: MatrixSummaryRequest,
    vector: Vec<f32>,
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
    let format = request.format.clone();
    matrix_json(&format, matrix_from_request(request)?)
}

fn matrix_stats_value(request: MatrixStatsRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request.matrix)?;
    let summary = matrix.summary().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "rows": matrix.rows(),
        "cols": matrix.cols(),
        "nnz": summary.nnz,
        "density": summary.density,
        "rowNnz": matrix.row_nnz(),
        "columnNnz": matrix.column_nnz(),
        "rowSums": matrix.row_sums().map_err(|error| error.to_string())?,
        "columnSums": matrix.column_sums().map_err(|error| error.to_string())?,
        "summary": {
            "rows": summary.rows,
            "cols": summary.cols,
            "nnz": summary.nnz,
            "density": summary.density,
            "rowNnzMin": summary.row_nnz_min,
            "rowNnzMax": summary.row_nnz_max,
            "rowNnzMean": summary.row_nnz_mean,
            "columnNnzMin": summary.column_nnz_min,
            "columnNnzMax": summary.column_nnz_max,
            "columnNnzMean": summary.column_nnz_mean
        }
    }))
}

fn vector_ops_value(request: VectorOpsRequest) -> Result<serde_json::Value, String> {
    let vector = sparse_vector(request.vector)?;
    let mut value = serde_json::json!({
        "dimensions": vector.dimensions(),
        "nnz": vector.nnz(),
        "l1Norm": vector.l1_norm().map_err(|error| error.to_string())?,
        "l2Norm": vector.l2_norm().map_err(|error| error.to_string())?
    });
    if let Some(scale) = request.scale {
        let scaled = vector.scale(scale).map_err(|error| error.to_string())?;
        value["scaled"] = vector_json(&scaled);
    }
    if let Some(add) = request.add {
        let added = vector
            .add(&sparse_vector(add)?)
            .map_err(|error| error.to_string())?;
        value["added"] = vector_json(&added);
    }
    if let Some(top_k) = request.top_k {
        value["topK"] = serde_json::json!(vector
            .top_k_by_abs(top_k)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|(index, value)| serde_json::json!({"index": index, "value": value}))
            .collect::<Vec<_>>());
    }
    Ok(value)
}

fn matrix_vector_value(request: MatrixVectorRequest) -> Result<serde_json::Value, String> {
    validate_value_count(request.vector.len())?;
    let matrix = matrix_from_request(request.matrix)?;
    let result = matrix
        .mul_dense_vector(&request.vector)
        .map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "rows": matrix.rows(),
        "cols": matrix.cols(),
        "values": result
    }))
}

fn transpose_value(request: MatrixSummaryRequest) -> Result<serde_json::Value, String> {
    let matrix = matrix_from_request(request)?;
    let transposed = matrix.transpose().map_err(|error| error.to_string())?;
    let coo = transposed.to_coo().map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "format": "coo",
        "rows": coo.rows(),
        "cols": coo.cols(),
        "entries": coo.entries()
    }))
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

fn matrix_from_request(request: MatrixSummaryRequest) -> Result<CsrMatrix, String> {
    match request.format.as_str() {
        "coo" => {
            validate_value_count(request.entries.len())?;
            CooMatrix::new(request.rows, request.cols, request.entries)
                .and_then(|coo| coo.to_csr())
                .map_err(|error| error.to_string())
        }
        "csr" => {
            validate_value_count(request.values.len())?;
            CsrMatrix::new(
                request.rows,
                request.cols,
                request.row_offsets,
                request.column_indices,
                request.values,
            )
            .map_err(|error| error.to_string())
        }
        format => Err(format!("unsupported sparse matrix format `{format}`")),
    }
}

fn vector_json(vector: &SparseVector) -> serde_json::Value {
    serde_json::json!({
        "dimensions": vector.dimensions(),
        "indices": vector.indices(),
        "values": vector.values(),
        "nnz": vector.nnz()
    })
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

fn default_sparse_format() -> String {
    "coo".to_string()
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

    #[test]
    fn new_sparse_operations_run() {
        for operation in [
            "sparse.vectorOps",
            "sparse.matrixVector",
            "sparse.transpose",
            "sparse.matrixStats",
        ] {
            let surface_operation = package_surface()
                .operations
                .into_iter()
                .find(|candidate| candidate.id.as_str() == operation)
                .expect("operation metadata");
            let response = run_surface_operation(SurfaceRequest {
                operation: surface_operation.id,
                input: surface_operation.example_request,
            })
            .unwrap_or_else(|error| panic!("{operation} failed: {error}"));
            assert!(response.value.is_object());
        }
    }

    #[test]
    fn sparse_matrix_stats_reports_columns_and_sums() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("sparse.matrixStats"),
            input: serde_json::json!({
                "matrix": {
                    "rows": 3,
                    "cols": 4,
                    "entries": [[0, 1, 2.0], [1, 3, 4.0], [2, 1, -1.0]]
                }
            }),
        })
        .expect("matrix stats");
        assert_eq!(response.value["nnz"], 3);
        assert_eq!(response.value["rowNnz"], serde_json::json!([1, 1, 1]));
        assert_eq!(response.value["columnNnz"], serde_json::json!([0, 2, 0, 1]));
        assert_eq!(
            response.value["rowSums"],
            serde_json::json!([2.0, 4.0, -1.0])
        );
    }
}
