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

**Best-Effort Result**:
An output whose shape is stable but whose quality depends on heuristics, local models, language coverage, fixtures, and backend availability.
_Avoid_: Production-grade NLP result, ground truth

**Model-Capable Text Crate**:
A text crate whose domain naturally supports both deterministic execution and explicit local-model-backed execution.
_Avoid_: AI crate, model-only crate

**Analytical Math Crates**:
The peer crate family under `crates/math` that provides reusable deterministic math primitives and package surfaces for multimodal workflows.
_Avoid_: Numerical Backend, Math Foundation Crates
