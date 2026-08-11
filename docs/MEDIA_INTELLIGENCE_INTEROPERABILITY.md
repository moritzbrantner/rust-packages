# Media Intelligence Interoperability

This document defines a JSON boundary between Rust capability operations and
the Python services that use `mi_contracts`. It does not create a shared source
dependency or move either side's semantic records into the other project.

The transport framing schema is
[`schemas/media-intelligence/v1/operation-envelope.schema.json`](../schemas/media-intelligence/v1/operation-envelope.schema.json),
identified by
`urn:moenarch:interoperability:media-intelligence:operation-envelope:1`.

## Ownership boundary

The operation envelope owns transport framing only: version, message kind,
message/correlation/causation ids, routing operation id, payload schema id,
payload slot, and optional occurrence time. It does not own the meaning or
shape of the payload.

| Contract | Semantic owner | Checked compatibility artifact |
| --- | --- | --- |
| Operation transport framing v1 | This repository's interoperability boundary until a reviewed release moves it | `schemas/media-intelligence/v1/operation-envelope.schema.json` |
| `SurfaceRequest`, `SurfaceResponse`, `Diagnostic`, and `SurfaceError` | `runtime-core` | Source-true payload snapshots used by the selected exchange |
| `OperationResult<T>` and `ArtifactRef` | `jobs-core` | Preserved inside the source `SurfaceResponse`; never relocated into framing |
| `TextStatisticsRequest` and `TextStatisticsResult` | `text-core` | `schemas/media-intelligence/v1/payloads/text-statistics-*.schema.json` |
| Python business/service records and `EventEnvelopeV1` | `mi_contracts` in `moritzbrantner/media-intelligence` | Pinned downstream schema snapshot under `schemas/media-intelligence/external/mi-contracts/0.1.0/` |
| File publication, HTTP routes/authentication, queue configuration, retries, deduplication, acknowledgement, and dead-letter policy | The deploying media-intelligence service | Downstream service configuration and tests |
| Truth Engine application records | `backend-contracts` | The independent Truth Engine package |

The checked schemas are compatibility snapshots, not a transfer of semantic
ownership. Rust source at commit
`364627c233b314807ba4f21298ada4cf63333bed` is authoritative for the Rust
serializations. The `mi_contracts` snapshot comes from media-intelligence commit
`e5b49cdd32acbfdaca057dc05d12412899f3129d`, package version `0.1.0`.
Changing a snapshot requires reconciling its owner first.

`runtime-core` and `jobs-core` remain domain-neutral and do not absorb Python
business records, persistence models, prompts, providers, or orchestration
state. `backend-contracts` remains Truth Engine-specific and is neither an owner
nor a dependency of this exchange.

## Selected operation exchange

The executable fixture selects `text.statistics` as one representative
capability. Its adapter mapping is deliberately narrow:

| Message | `payloadSchema` | Exact payload and adapter behavior |
| --- | --- | --- |
| `request` | `urn:moenarch:text-core:text-statistics-request:0.1.0` | `payload` is the serialized `TextStatisticsRequest`. The adapter constructs `SurfaceRequest { operation: operationId, input: payload }`; it does not reinterpret a `mi_contracts` business record as a Rust DTO. |
| `result` | `urn:moenarch:text-core:text-statistics-surface-response:0.1.0` | `payload` is the complete serialized `SurfaceResponse` returned by `text_core::surface::run_surface_operation`. The adapter does not unwrap, relocate, merge, or deduplicate evidence channels. |
| `error` | `urn:moenarch:runtime-core:surface-error:0.2.0` | `payload` is the exact `SurfaceError` decoded from the Rust error string. The framing `operationId` still routes/correlates the failed request; `SurfaceError.operation` remains source-owned and may be `null`. |

The operation envelope has no `diagnostics` or `artifacts` fields. Within a
result payload, the outer `SurfaceResponse` channels and any nested
`OperationResult<T>` channels retain their existing owners and positions. There
is no interoperability-level precedence or merge rule: consumers preserve the
full payload and apply the runtime/jobs contract semantics at their original
locations. The fixture smoke checks repeated structured-response projections
for consistency without defining a new DTO.

An adapter allowlists exact `(messageType, operationId, payloadSchema)` tuples.
It rejects an unknown tuple even when the JSON resembles a known payload. A new
operation or incompatible payload schema requires a separately reviewed tuple.

## Relationship to `mi_contracts.events.EventEnvelopeV1`

The current `EventEnvelopeV1` contains event metadata and an optional
`payload_location`; it has no inline body/payload field. Therefore it is not the
operation envelope and does not directly contain one.

For media-intelligence queues that use `EventEnvelopeV1`, the selected mapping
is a pointer wrapper:

| `EventEnvelopeV1` field | Operation-envelope relationship |
| --- | --- |
| `schema_version` and `metadata.schema_version` | Both must be `v1` for operation `schemaVersion: 1`. Other version pairs require a new reviewed mapping. |
| `event_id` | Equals the pointed-to operation envelope's `messageId`. |
| `event_type` | `rust.operation.<messageType>`, currently `rust.operation.request`, `rust.operation.result`, or `rust.operation.error`. |
| `timestamp` | Equals the operation envelope's required `occurredAt` for this queue adapter. |
| `source_id` | Deploying service instance/source identifier; it is not a Rust contract id. |
| `payload_location` | Non-null immutable location of the JSON operation envelope. The adapter loads and validates that object and its payload schema. |
| `metadata.source_type` | `rust-capability-operation-envelope`. |
| `metadata.retry_count` | Service-owned delivery metadata; it does not alter `messageId`. |

The checked event fixture proves this pointer relationship. No direct-body
mapping is selected because the downstream contract has no field for it.
Implementing the loader, storage access, and queue policy remains downstream
media-intelligence work; this repository can validate the contract without
claiming that deployment work already exists.

## Transport mappings and service requirements

| Transport | Selected mapping |
| --- | --- |
| File | One UTF-8 operation envelope per JSON object/file. Envelope and payload schemas are validated before adapter conversion. |
| HTTP | The request body is the operation envelope with `application/json`; response bodies are result or error operation envelopes. HTTP status, auth, tracing, and route versions stay transport-owned. |
| Existing media-intelligence queue | The message body is `mi_contracts.events.EventEnvelopeV1`; its `payload_location` points to the operation-envelope JSON as defined above. |

Atomic file publication, immutable object writes, message deduplication,
redelivery behavior, acknowledgement timing, and dead-letter routing are
requirements the deploying service must confirm and implement before production
use. They are not frozen by this schema. A service adapter test should document
its actual guarantees, including what happens after envelope validation,
payload validation, storage failure, retry, and duplicate delivery.

## Compatibility rules

- Consumers reject an unsupported envelope `schemaVersion` before inspecting
  `payload`.
- Required framing fields and existing `messageType` meanings do not change
  within v1. Optional framing fields may be added; v1 consumers ignore unknown
  framing fields after validation.
- A breaking framing change uses a new directory, schema id, and integer
  `schemaVersion`.
- `payloadSchema` is an exact versioned identifier. The adapter resolves it from
  an explicit schema registry and validates `payload` before conversion.
- Rust and Python payload compatibility remains independently owned. A breaking
  owner change receives a new schema identifier and allowlist entry.
- An existing `operationId` is not reused for incompatible semantics.
- Replies use the request's `correlationId` and set `causationId` to the request
  `messageId`. A reply with a mismatch is rejected.
- One fixture exchange contains exactly one request, one result, and one error.
  Duplicate message kinds are rejected by the compatibility smoke.

A media-intelligence clean checkout needs immutable JSON Schema artifacts and
Python dependencies only. It never needs a Rust checkout, Cargo path dependency,
generated Rust library, or sibling repository path. Local source-truth checks
may execute Rust in this repository, but production dependency resolution may
not depend on that checkout.

## Executable contract smoke

Run the local positive and negative compatibility suite (Python `jsonschema`
4.x is required):

```bash
python3 tests/fixtures/media-intelligence/v1/compatibility_smoke.py
```

It discovers the operation fixture set from disk and fails closed on any
unknown, duplicate, or incomplete request/result/error fixture set. It validates
every discovered operation envelope, resolves and validates every
`payloadSchema`, validates the downstream `EventEnvelopeV1` pointer fixture,
loads the pointed-to `fixture://` object inside the approved v1 fixture root,
validates that operation envelope and payload, checks source-truth projections,
and proves rejection of:

- unknown allowlist tuples;
- mismatched causation or correlation;
- the wrong error payload schema;
- missing framing fields and malformed payloads;
- duplicate message kinds and extra on-disk operation fixtures;
- unresolved payload-schema identifiers; and
- missing, unauthorized, escaping, nonexistent, and mismatched pointer targets.

Confirm the Rust source serialization used by the fixtures with:

```bash
cargo run -q -p moenarch-text-core-cli -- \
  run --operation text.statistics --json '{"text":"Hello world. Again."}'
cargo run -q -p moenarch-text-core-cli -- \
  run --operation text.statistics --json '{"missing":true}' 2>&1
```

The actual downstream Pydantic model check is executable without a committed
path dependency. With authenticated access, clone and pin media-intelligence to
`e5b49cdd32acbfdaca057dc05d12412899f3129d`, install only its contracts package
into a temporary virtual environment, and validate the checked pointer fixture
with `mi_contracts.events.EventEnvelopeV1.model_validate`:

```bash
mi_contracts_check="$(mktemp -d -t mi-contracts-check.XXXXXX)"
gh repo clone moritzbrantner/media-intelligence \
  "$mi_contracts_check/media-intelligence"
git -C "$mi_contracts_check/media-intelligence" checkout --detach \
  e5b49cdd32acbfdaca057dc05d12412899f3129d
python3 -m venv "$mi_contracts_check/venv"
"$mi_contracts_check/venv/bin/python" -m pip install \
  "$mi_contracts_check/media-intelligence/contracts"
"$mi_contracts_check/venv/bin/python" \
  tests/fixtures/media-intelligence/v1/mi_contracts_model_smoke.py \
  --checkout "$mi_contracts_check/media-intelligence"
```

The runner verifies both the exact commit and the GitHub origin before importing
the downstream model. It then requires an exact Pydantic round trip including
`payload_location`. This is a test-only temporary checkout and is not a
production, Cargo, Python, or committed sibling-path dependency.

Also detect current `EventEnvelopeV1` schema drift from the pinned snapshot:

```bash
gh api -H 'Accept: application/vnd.github.raw+json' \
  'repos/moritzbrantner/media-intelligence/contents/contracts/schemas/v1/event_envelope_v1.json?ref=main' |
python3 -c 'import json, sys; from pathlib import Path; remote=json.load(sys.stdin); local=json.loads(Path("schemas/media-intelligence/external/mi-contracts/0.1.0/event-envelope-v1.schema.json").read_text()); assert remote == local; print("mi_contracts EventEnvelopeV1 snapshot matches current main")'
```

The contract/model/pointer integration is tested here. Deployment wiring for
object storage, queue delivery, acknowledgements, retries, and service-specific
adapter registration remains downstream media-intelligence rollout work; this
slice does not claim that operational deployment is complete.
