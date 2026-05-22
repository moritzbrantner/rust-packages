# text-nlp-models

Shared request and response schemas, model metadata, and lightweight fallback
runners for text NLP tasks used by the Rust, CLI, server, UI, and WASM
surfaces.

Large transformer inference is intentionally not bundled into browser/WASM
surfaces. Native model execution can be added behind the same schemas while
fallback and imported-prediction paths remain stable.

