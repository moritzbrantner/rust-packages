# text-nlp-models

Compatibility crate for the text NLP task surface. New code should use
`text-nlp-tasks`; this package keeps the previous crate name available while
the workspace migrates.

Large transformer inference is intentionally not bundled into browser/WASM
surfaces. Native model execution can be added behind the same schemas while
fallback and imported-prediction paths remain stable.
