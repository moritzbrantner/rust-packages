# Rust Multimodal Analysis Packages

This context defines the project language for the Rust-first multimodal package workspace and its package consumer release work.

## Language

**Package Consumer**:
An engineer who imports the Rust crates or their adapter contracts to build an application or workflow.
_Avoid_: End user, app user

**Composable Building Block**:
A crate capability exposed through stable library and adapter contracts so external projects can use it in workflows without the crate knowing about the workflow graph.
_Avoid_: Workflow node, graph node, ComfyUI node

**Contract Owner**:
The crate that owns the most general semantic form of a shared type and defines compatibility rules for specialized crates.
_Avoid_: Duplicate DTO owner, local schema copy

**Adapter Parity**:
The guarantee that library, CLI, REST, WASM, and web app surfaces delegate to the same library-owned behavior and preserve the same request and response contract.
_Avoid_: Demo wrapper parity, transport-specific behavior

**Runtime Surface**:
A stable operation contract that lets a crate expose the same request and response shape across adapters.
_Avoid_: Endpoint, demo API

**Foundation Release Wave**:
A publishable group of foundational crates that downstream package consumers can build on.
_Avoid_: Foundation sprint, base cleanup

**Model Runtime Spine**:
The first foundation slice centered on runtime DTOs, job and artifact contracts, model bundle lifecycle, and opt-in ONNX execution.
_Avoid_: Model platform, inference layer

**Native Workflow**:
A workflow that requires local tools, downloaded model artifacts, native runtimes, or network/materialization setup.
_Avoid_: Default workflow, local-first workflow

**Model Bundle**:
A local set of model files and metadata prepared for a caller or task crate to use.
_Avoid_: Model cache, weights folder

**Stable Contract**:
Public API, schema, runtime operation envelope, adapter behavior, and compatibility rules promised by a release.
_Avoid_: Accuracy guarantee, model quality promise

**Visual Detection**:
A localized visual finding with kind, region, score, optional keypoints, and optional metadata. It is not a persistent entity and does not imply identity.
_Avoid_: Identity, tracked entity, generic bounding box

**Visual Keypoint**:
A point attached to a visual detection, optionally named and scored.
_Avoid_: Landmark-only DTO, pose joint

**Visual Embedding**:
A dense vector representation of an image, region, or detection, optionally linked to its visual source.
_Avoid_: Identity record, reference profile

**Identity Match**:
A scored hypothesis linking a visual detection or embedding to a reference identity.
_Avoid_: Ground truth identity, persistent person

**Best-Effort Result**:
An output whose shape is stable but whose quality depends on heuristics, local models, language coverage, fixtures, and backend availability.
_Avoid_: Production-grade NLP result, ground truth

**Model-Capable Text Crate**:
A text crate whose domain naturally supports both deterministic execution and explicit local-model-backed execution.
_Avoid_: AI crate, model-only crate

**Text Corpus**:
A raw collection of text documents or segments with stable ids, language, source, provenance, annotations, and metadata before indexing.
_Avoid_: Search index, knowledge base

**Text Index**:
A searchable representation of text corpus content, including chunks, lexical state, vectors, facets, and persistence metadata.
_Avoid_: Corpus, document store

**Contract Ingestion**:
Conversion from existing text contracts, transcript segments, OCR outputs, and plain text records into indexable documents/chunks.
_Avoid_: File extraction, document parsing

**Semantic Facet**:
A searchable/filterable meaning-bearing label derived from analysis, such as entity, topic, relation, classification label, language, source, or provenance.
_Avoid_: Knowledge graph claim, ground truth

**Analytical Math Crates**:
The peer crate family under `crates/math` that provides reusable deterministic math primitives and package surfaces for multimodal workflows. It owns generic dense linear algebra, statistics, sparse data, signal kernels, and map kernels, not 3D coordinate-system semantics.
_Avoid_: Numerical Backend, Math Foundation Crates, 3D Spatial Core

**Three-D Spatial Core**:
The crate family centered on `three-d-processing-core` that owns workspace 3D vectors, points, rotations, transforms, camera geometry, and coordinate-convention conversions.
_Avoid_: Math crate, graphics helper crate, radiance-only geometry

**Adjacent Domain Package Family**:
A useful, coherent package family whose domain can support multimodal workflows but whose primary purpose belongs outside this repository's core video, audio, image, text, vector, animation, 3D, runtime, and interoperability scope.
_Avoid_: Failed package, low-quality crate, unrelated code
