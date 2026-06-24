# ComfyUI Type Matrix

This matrix is the checked-in source of truth for how ComfyUI-shaped data maps
onto workspace crates. It combines:

- Official ComfyUI datatype docs:
  [Datatypes](https://docs.comfy.org/custom-nodes/backend/datatypes)
  and [Nodes](https://docs.comfy.org/development/core-concepts/nodes)
- Current workspace contracts in
  [API_CONTRACTS.md](API_CONTRACTS.md)
  and [src/lib.rs](/home/moenarch/moritzbrantner/rust-packages/src/lib.rs)
- Current local ComfyUI usage in
  [image-analysis-comfyui](/home/moenarch/moritzbrantner/rust-packages/crates/image/image-analysis-comfyui/src/lib.rs)
  and [comfyui-workflow.json](/home/moenarch/moritzbrantner/rust-packages/tests/fixtures/comfyui-workflow.json)

## Matrix

| Type | Observed Locally | Portability | Current Owner | Phase 1 Action | Notes |
| --- | --- | --- | --- | --- | --- |
| `INT` | No | Portable | `moenarch-numbers-core` + workflow typing | Existing | Scalar workflow/config data stays out of Comfy-specific crates. |
| `FLOAT` | No | Portable | `moenarch-numbers-core` + workflow typing | Existing | Scalar workflow/config data stays out of Comfy-specific crates. |
| `STRING` | No | Portable | workflow typing | Existing | String widget/config semantics stay in workflow typing layers. |
| `BOOLEAN` | No | Portable | workflow typing | Existing | Boolean widget/config semantics stay in workflow typing layers. |
| `COMBO` | No | Portable | workflow typing | Existing | Represents UI choice/config, not a reusable media package. |
| `IMAGE` | Yes | Portable | `moenarch-image-analysis-core` | Existing + batch wrappers | `ImageBatchView` and `OwnedImageBatch` own batched image semantics. |
| `MASK` | Yes | Portable | `moenarch-image-analysis-core` + `moenarch-tensor-data` | Existing + bridge helper | `mask_tensor_from_luma` bridges masks into tensor form without a separate mask crate. |
| `AUDIO` | No | Portable | `moenarch-audio-analysis-core` + `moenarch-tensor-data` | Existing + batch wrappers | `OwnedAudioWaveformBatch` and `AudioWaveformBatchView` own `[B,C,T]` waveform semantics. |
| `VIDEO` | No | Asset-oriented | video/workflow crates | Existing | Phase 1 keeps video asset-oriented; no in-memory video tensor package is introduced. |
| `LATENT` | Yes | Portable | `moenarch-comfyui-latents` + `moenarch-tensor-data` | New crate | `LatentBatch` owns validated `[B,C,H,W]` latent tensors and optional masks. |
| `MODEL` | Yes | Mixed | `moenarch-comfyui-data` + `moenarch-comfyui-models` | Semantic inventory + model refs | Typed socket inventory is stable now; opaque runtime payload schema remains abstract. |
| `CLIP` | Yes | Runtime-leaning | `moenarch-comfyui-data` | Semantic inventory only | Represent as a typed socket category; concrete runtime schema is deferred. |
| `CLIP_VISION` | No | Mixed | `moenarch-comfyui-data` + `moenarch-comfyui-models` | Semantic inventory + model refs | Asset lookup is stable through `ComfyModelRole::ClipVision`. |
| `VAE` | Yes | Mixed | `moenarch-comfyui-data` + `moenarch-comfyui-models` | Semantic inventory + model refs | VAE asset refs are stable; runtime payload remains abstract. |
| `CONDITIONING` | Yes | Minimal runtime contract | `moenarch-comfyui-data` + `moenarch-tensor-data` | Minimal tensor-backed runtime schema | `ConditioningBatch` owns stable `[T,C]` embedding tensors plus optional pooled `[C]` embeddings. |
| `UPSCALE_MODEL` | Yes | Mixed | `moenarch-comfyui-data` + `moenarch-comfyui-models` | Semantic inventory + model refs | Asset lookup is stable through `ComfyModelRole::UpscaleModel`. |
| `MODEL_PATCH` / `MODELPATCH` | No | Mixed | `moenarch-comfyui-data` + `moenarch-comfyui-models` | Semantic inventory + model refs | Asset lookup is stable through `ComfyModelRole::ModelPatch`. |
| `MESH` | No | Portable | `three-d-processing-*` + radiance crates | Existing | Mesh/radiance-like data already belongs to 3D and radiance packages. |
| `NOISE` | No | Runtime-only | `moenarch-comfyui-data` | Deferred runtime schema | Tracked only as a socket category until a stable reusable representation is needed. |
| `SAMPLER` | No | Runtime-only | `moenarch-comfyui-data` | Deferred runtime schema | Tracked only as a socket category until a stable reusable representation is needed. |
| `SIGMAS` | No | Runtime-only | `moenarch-comfyui-data` | Deferred runtime schema | Tracked only as a socket category until a stable reusable representation is needed. |
| `GUIDER` | No | Runtime-only | `moenarch-comfyui-data` | Deferred runtime schema | Tracked only as a socket category until a stable reusable representation is needed. |

## Ownership Rules

- `moenarch-comfyui-data` owns workflow graph structure and normalized socket inventory.
- `moenarch-tensor-data` owns generic finite `f32` tensor storage and metadata.
- `moenarch-comfyui-latents` owns ComfyUI-flavored latent tensors and latent masks.
- `moenarch-comfyui-models` owns model folder kinds and stable model-reference contracts.
- `moenarch-image-analysis-core` owns image buffers and image batches.
- `moenarch-audio-analysis-core` owns waveform batches.
- `moenarch-comfyui-data` owns the minimal tensor-backed `ConditioningBatch` contract.
- Runtime-only sockets such as `NOISE`, `SAMPLER`, `SIGMAS`, and `GUIDER`
  remain intentionally schema-light until a second concrete producer/consumer
  pair appears in the workspace.
