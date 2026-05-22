# audio-analysis-models

Shared request and response schemas, model metadata, and lightweight fallback
runners for audio model tasks used by Rust, CLI, server, and UI surfaces.

Large transformer inference is intentionally not bundled into default package
apps. Native model execution can be added behind these schemas while fallback
and imported-prediction paths remain stable.

