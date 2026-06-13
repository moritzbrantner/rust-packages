//! Library-owned runtime surface for `text-index`.

use runtime_core::{
    structured_surface_value, OperationId, PackageSurface, RuntimeCapabilities, SurfaceOperation,
    SurfaceRequest, SurfaceResponse,
};
use serde::Deserialize;
use text_embeddings::{HashedTextEmbedder, TextEmbeddingConfig};
use text_lexical::CorpusOptions;

use crate::{
    IndexBuildOptions, IndexDocument, IndexMutationReport, IndexQuery, MemoryIndexStore, TextIndex,
};

pub fn package_surface() -> PackageSurface {
    PackageSurface {
        library: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        capabilities: RuntimeCapabilities::pure_rust(),
        operations: vec![
            operation("describe", "Inspect package metadata", "Durable local text indexing and hybrid search.", serde_json::json!({"includeOperations": true})),
            operation("index.build", "Build index", "Builds a transient in-memory text index unless a committed SQLite surface is used.", serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text index"}]})),
            operation("index.open", "Open index", "Opens or describes an index backend.", serde_json::json!({"backend": "memory"})),
            operation("index.addDocuments", "Add documents", "Adds documents to a transient in-memory index for package-surface execution.", serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text index"}]})),
            operation("index.removeDocuments", "Remove documents", "Plans document removals for package-surface execution.", serde_json::json!({"documentIds": ["doc-1"]})),
            operation("index.search", "Search index", "Builds a transient in-memory index and searches it.", serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text index"}], "query": {"text": "text index"}})),
            operation("index.inspect", "Inspect index", "Builds a transient in-memory index and returns counts.", serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text index"}]})),
            operation("index.snapshotPlan", "Plan snapshot", "Builds a transient index and returns a dry-run snapshot plan.", serde_json::json!({"documents": [{"id": "doc-1", "body": "Rust text index"}]})),
        ],
    }
}

pub fn run_surface_operation(request: SurfaceRequest) -> Result<SurfaceResponse, String> {
    let operation = request.operation.clone();
    let value = match request.operation.as_str() {
        "describe" | "index.open" => describe_value(request.input),
        "index.build" | "index.addDocuments" => {
            let input: BuildRequest = parse_input(request.input)?;
            let (index, report) = build_index(&input)?;
            serde_json::json!({"report": report, "inspect": index.inspect().map_err(|error| error.to_string())?})
        }
        "index.removeDocuments" => {
            let input: RemoveRequest = parse_input(request.input)?;
            serde_json::json!({"accepted": true, "documentIds": input.document_ids, "commitRequiredForDurableWrites": true})
        }
        "index.search" => {
            let input: SearchRequest = parse_input(request.input)?;
            let (index, _) = build_index(&BuildRequest {
                documents: input.documents,
                options: input.options,
                dimensions: input.dimensions,
            })?;
            serde_json::json!({"results": index.search(&input.query).map_err(|error| error.to_string())?})
        }
        "index.inspect" => {
            let input: BuildRequest = parse_input(request.input)?;
            let (index, _) = build_index(&input)?;
            serde_json::json!(index.inspect().map_err(|error| error.to_string())?)
        }
        "index.snapshotPlan" => {
            let input: BuildRequest = parse_input(request.input)?;
            let (index, _) = build_index(&input)?;
            serde_json::json!(index.snapshot_plan().map_err(|error| error.to_string())?)
        }
        other => {
            return Err(runtime_core::SurfaceError::unsupported_operation(
                other,
                env!("CARGO_PKG_NAME"),
            )
            .to_error_string())
        }
    };
    Ok(SurfaceResponse {
        operation: operation.clone(),
        value: annotated_value(&operation, value),
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
    })
}

fn build_index(
    input: &BuildRequest,
) -> Result<
    (
        TextIndex<HashedTextEmbedder, MemoryIndexStore>,
        IndexMutationReport,
    ),
    String,
> {
    let embedder = HashedTextEmbedder::new(
        TextEmbeddingConfig {
            dimensions: input.dimensions.max(1),
            use_idf: false,
        },
        CorpusOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let mut index = TextIndex::with_store(embedder, MemoryIndexStore::new())
        .with_options(input.options.clone());
    let report = index
        .upsert_documents(&input.documents)
        .map_err(|error| error.to_string())?;
    Ok((index, report))
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
        "describe" | "index.open" => (
            "Package surface metadata",
            "Inspected the text-index package operations and runtime support.",
            serde_json::json!({
                "status": "ok",
                "operationCount": value["operationCount"]
            }),
        ),
        "index.build" | "index.addDocuments" => (
            "Index build result",
            "Built a transient in-memory text index from the provided documents.",
            serde_json::json!({
                "status": "ok",
                "documentsUpserted": value["report"]["documentsUpserted"],
                "chunkCount": value["inspect"]["chunkCount"],
                "vectorCount": value["inspect"]["vectorCount"]
            }),
        ),
        "index.removeDocuments" => (
            "Index removal plan",
            "Planned document removal for a durable text index; browser package execution stays side-effect free.",
            serde_json::json!({
                "status": "ok",
                "documentCount": value["documentIds"].as_array().map(Vec::len).unwrap_or(0),
                "commitRequiredForDurableWrites": value["commitRequiredForDurableWrites"]
            }),
        ),
        "index.search" => (
            "Index search result",
            "Built a transient in-memory text index and searched it with deterministic lexical and semantic scoring.",
            serde_json::json!({
                "status": "ok",
                "resultCount": value["results"].as_array().map(Vec::len).unwrap_or(0)
            }),
        ),
        "index.inspect" => (
            "Index inspection result",
            "Inspected transient text index counts without durable writes.",
            serde_json::json!({
                "status": "ok",
                "documentCount": value["documentCount"],
                "chunkCount": value["chunkCount"],
                "vectorCount": value["vectorCount"],
                "facetCount": value["facetCount"]
            }),
        ),
        "index.snapshotPlan" => (
            "Index snapshot plan",
            "Planned transient index snapshot metadata without writing files.",
            serde_json::json!({
                "status": "ok",
                "backend": value["backend"],
                "documentCount": value["documentCount"],
                "chunkCount": value["chunkCount"],
                "vectorCount": value["vectorCount"]
            }),
        ),
        _ => (
            "Text index result",
            "Ran a text-index package operation.",
            serde_json::json!({"status": "ok"}),
        ),
    };
    structured_surface_value(operation, title, message, summary, value)
}

fn operation(
    id: &str,
    name: &str,
    description: &str,
    example_request: serde_json::Value,
) -> SurfaceOperation {
    runtime_core::surface_operation(id, name, description, example_request)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildRequest {
    documents: Vec<IndexDocument>,
    #[serde(default)]
    options: IndexBuildOptions,
    #[serde(default = "default_dimensions")]
    dimensions: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    documents: Vec<IndexDocument>,
    query: IndexQuery,
    #[serde(default)]
    options: IndexBuildOptions,
    #[serde(default = "default_dimensions")]
    dimensions: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveRequest {
    document_ids: Vec<String>,
}

fn parse_input<T: for<'de> Deserialize<'de>>(input: serde_json::Value) -> Result<T, String> {
    serde_json::from_value(input).map_err(|error| format!("invalid request: {error}"))
}

fn default_dimensions() -> usize {
    128
}
