//! Library-owned runtime surface for `text-retrieval`.

use serde::Deserialize;
use text_core::TextProcessingOptions;
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use video_analysis_core::runtime::{
    structured_surface_value, OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation,
    SurfaceRequest, SurfaceResponse,
};

use crate::{
    chunk_search_document, rerank_documents, ChunkingOptions, IngestReport, IngestionOptions,
    RerankRequest, RetrievalIndex, RetrievalMode, SearchDocument, SearchFilter, SearchQuery,
};

/// Returns the package surface exposed by every transport wrapper.
pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation(
                "describe",
                "Inspect package metadata",
                "Library-first semantic and hybrid retrieval for video-analysis.",
                serde_json::json!({"includeOperations": true}),
            ),
            operation(
                "retrieval.chunk",
                "Chunk documents",
                "Chunks search documents without writing persistence artifacts.",
                serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text retrieval. Hybrid search."}]}),
            ),
            operation(
                "retrieval.search",
                "Search documents",
                "Builds a transient in-memory retrieval index and searches it.",
                serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text retrieval"}, {"id": "doc-2", "body": "Video scene reports"}], "query": "text", "mode": "hybrid"}),
            ),
            operation(
                "retrieval.rerank",
                "Rerank documents",
                "Reranks query/document pairs using imported scores or deterministic lexical overlap.",
                serde_json::json!({"query": "rust", "documents": ["rust text", "video scenes"]}),
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
        "retrieval.chunk" => chunk_value(parse_input(request.input)?)?,
        "retrieval.search" => search_value(parse_input(request.input)?)?,
        "retrieval.rerank" => serde_json::to_value(
            rerank_documents(parse_input::<RerankRequest>(request.input)?)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?,
        operation => {
            return Err(format!(
                "unsupported operation `{operation}` for {}",
                env!("CARGO_PKG_NAME")
            ))
        }
    };
    let value = annotated_value(&operation, value);
    Ok(SurfaceResponse {
        operation,
        value,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    })
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

fn annotated_value(operation: &OperationId, value: serde_json::Value) -> serde_json::Value {
    let (title, message, summary) = match operation.as_str() {
        "describe" => (
            "Package surface metadata",
            "Inspected the text-retrieval package operations and runtime support.",
            serde_json::json!({
                "status": "ok",
                "operationCount": value["operationCount"]
            }),
        ),
        "retrieval.chunk" => (
            "Document chunking result",
            "Chunked search documents in memory without writing persistence artifacts.",
            serde_json::json!({
                "status": "ok",
                "documentCount": value["report"]["documentsReceived"],
                "chunkCount": value["chunks"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "retrieval.search" => (
            "Retrieval search result",
            "Built a transient in-memory retrieval index and searched it.",
            serde_json::json!({
                "status": "ok",
                "mode": value["mode"],
                "indexedChunks": value["report"]["chunksIndexed"],
                "resultCount": value["results"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "retrieval.rerank" => (
            "Document reranking result",
            "Reranked query/document pairs using imported scores or deterministic lexical overlap.",
            serde_json::json!({
                "status": "ok",
                "query": value["query"],
                "resultCount": value["results"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        _ => (
            "Text retrieval result",
            "Ran a text-retrieval package operation.",
            serde_json::json!({"status": "ok"}),
        ),
    };
    structured_surface_value(operation, title, message, summary, value)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChunkRequest {
    documents: Vec<SearchDocument>,
    #[serde(default)]
    options: ChunkingOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RetrievalSearchRequest {
    documents: Vec<SearchDocument>,
    query: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_top_k")]
    top_k: usize,
    #[serde(default)]
    filters: Vec<SearchFilter>,
    #[serde(default = "default_dimensions")]
    dimensions: usize,
}

fn chunk_value(request: ChunkRequest) -> Result<serde_json::Value, String> {
    let processing = TextProcessingOptions::default();
    let mut chunks = Vec::new();
    for document in &request.documents {
        chunks.extend(
            chunk_search_document(document, &request.options, &processing)
                .map_err(|error| error.to_string())?,
        );
    }
    let report = IngestReport {
        documents_received: request.documents.len(),
        documents_replaced: 0,
        documents_skipped: request
            .documents
            .iter()
            .filter(|document| document.body.trim().is_empty())
            .count(),
        chunks_indexed: chunks.len(),
    };
    Ok(serde_json::json!({ "chunks": chunks, "report": report }))
}

fn search_value(request: RetrievalSearchRequest) -> Result<serde_json::Value, String> {
    let mut index = RetrievalIndex::new(HashedTextEmbedder {
        config: TextEmbeddingConfig {
            dimensions: request.dimensions.max(1),
            use_idf: false,
        },
        ..HashedTextEmbedder::default()
    });
    let report = index
        .ingest_documents(&request.documents, &IngestionOptions::default())
        .map_err(|error| error.to_string())?;
    let mut query = SearchQuery::new(request.query, request.top_k).mode(parse_mode(&request.mode)?);
    if let Some(filter) = request.filters.into_iter().next() {
        query = query.filter(filter);
    }
    let results = index.search(&query).map_err(|error| error.to_string())?;
    Ok(serde_json::json!({
        "mode": format!("{:?}", query.retrieval_mode()),
        "report": report,
        "results": results
    }))
}

fn parse_mode(mode: &str) -> Result<RetrievalMode, String> {
    match mode {
        "full_text" | "fullText" | "full-text" => Ok(RetrievalMode::FullText),
        "semantic" => Ok(RetrievalMode::Semantic),
        "hybrid" => Ok(RetrievalMode::Hybrid),
        other => Err(format!("unsupported retrieval mode `{other}`")),
    }
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_mode() -> String {
    "hybrid".to_string()
}
fn default_top_k() -> usize {
    10
}
fn default_dimensions() -> usize {
    128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_surface_lists_retrieval_operations() {
        let ids = package_surface()
            .operations
            .into_iter()
            .map(|operation| operation.id.0)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"retrieval.chunk".to_string()));
        assert!(ids.contains(&"retrieval.search".to_string()));
        assert!(ids.contains(&"retrieval.rerank".to_string()));
    }

    #[test]
    fn search_operation_returns_ranked_results() {
        let response = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("retrieval.search"),
            input: serde_json::json!({
                "documents": [
                    {"id": "doc-1", "body": "rust text retrieval"},
                    {"id": "doc-2", "body": "video scene reports"}
                ],
                "query": "text",
                "mode": "full_text"
            }),
        })
        .expect("search");
        assert_eq!(response.value["results"][0]["document_id"], "doc-1");
    }

    #[test]
    fn malformed_input_returns_typed_error_string() {
        let error = run_surface_operation(SurfaceRequest {
            operation: OperationId::new("retrieval.search"),
            input: serde_json::json!({"documents": []}),
        })
        .expect_err("invalid request");
        assert!(error.contains("invalid request"));
    }
}
