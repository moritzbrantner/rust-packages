# Rust Multimodal Analysis Packages

This context defines the project language for the Rust-first multimodal package workspace and its package consumer release work.

## Language

**Package Consumer**:
An engineer who imports the Rust crates or their adapter contracts to build an application or workflow.
_Avoid_: End user, app user

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
