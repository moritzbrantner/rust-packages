# Curated Landscape

The curated landscape is a lightweight metadata layer for package consumers. It
lets crates describe stable type and function relationships without adding a
workflow engine, graph model, or new dependency boundary.

`moritzbrantner-runtime-core` owns the metadata shape. Domain crates continue to
own the semantic contracts for their own types.

## Curated Types

A curated type is a stable identifier for a contract that package consumers can
use when wiring crates together. It points at an owner package and, when useful,
the Rust type name.

Examples:

- `text.document` owned by `moritzbrantner-text-core`
- `image.image` owned by `moritzbrantner-image-analysis-core`
- `vision.detection` owned by `moritzbrantner-vision-core`
- `tensor.f32Tensor` owned by `moritzbrantner-tensor-data`

The identifier is metadata. It does not make `runtime-core` depend on the owner
crate, and it does not replace the owner crate's Rust API.

## Curated Functions

A curated function describes what a runtime operation consumes and produces in
terms of curated types. It is attached to an existing `SurfaceOperation` through
the `xLandscape` schema extension:

```json
{
  "xLandscape": {
    "function": {
      "id": "image.core.summarizeImage",
      "owner": "moritzbrantner-image-analysis-core",
      "inputs": [
        {
          "name": "image",
          "typeRef": {
            "id": "image.image",
            "owner": "moritzbrantner-image-analysis-core",
            "rustType": "image_analysis_core::OwnedImage"
          },
          "required": true,
          "cardinality": "one"
        }
      ],
      "outputs": [
        {
          "name": "summary",
          "typeRef": {
            "id": "numbers.summary",
            "owner": "moritzbrantner-numbers-core",
            "rustType": "numbers_core::NumberSummary"
          },
          "required": true,
          "cardinality": "one"
        }
      ],
      "stability": "stable"
    }
  }
}
```

Crates attach this metadata with `runtime_core::attach_landscape_contract` or
`runtime_core::surface_operation_with_landscape`.

`xLandscape` is optional operation metadata. When a crate declares it, the
extension must be present and identical on both `input_schema` and
`output_schema` so transports and apps can read the same contract from either
side of the operation schema.

## Ownership

`runtime-core` owns only these shared metadata structs:

- `LandscapeTypeId`
- `LandscapeFunctionId`
- `LandscapeTypeRef`
- `LandscapePort`
- `LandscapeFunction`
- `LandscapeOperationContract`

It also owns generic validation for non-empty IDs, known owner packages, known
well-known type IDs, port names, and required input/output declarations.

The owner package named inside each `LandscapeTypeRef` owns the actual contract
semantics and compatibility rules. Specialized crates may enrich a contract, but
they must preserve a conversion path back to the owner contract instead of
creating unrelated parallel DTOs.

## Not A Workflow Graph

The curated landscape is not a node graph, port system, layout model, or
execution planner. It is also not a node system, scheduler, runtime placement
model, or graph execution contract. It does not decide execution order or
runtime placement.

Use existing runtime surface metadata for those concerns:

- `SurfaceOperation` owns operation metadata.
- `xExecutionPlan` describes execution mode and side effects.
- `SurfaceRequest` and `SurfaceResponse` remain the transport envelope.
- Package consumers may map curated types into their own graph model if they
  choose, but crates do not depend on that model.

## Compatibility Rules

- Curated type and function IDs are stable once released.
- Metadata fields are additive-only after release unless a migration is
  documented.
- Removals and renames require a documented migration.
- Missing `xLandscape` means the operation has not declared curated I/O yet; it
  does not mean the operation is not composable.
- Malformed declared metadata is a bug and should fail landscape validation.
- Domain crates own semantic compatibility for their types and conversions.
- `runtime-core` owns only the metadata shape, well-known IDs, and generic
  validation.

## Inventory

The inventory is generated:

```bash
python3 scripts/audit_curated_landscape.py --write
```

Check it without rewriting:

```bash
python3 scripts/audit_curated_landscape.py --check
```

See `docs/CURATED_LANDSCAPE_MATRIX.md` for the current foundation and workflow
rows.
