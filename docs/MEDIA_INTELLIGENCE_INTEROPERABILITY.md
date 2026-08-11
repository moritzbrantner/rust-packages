# Media Intelligence Interoperability

This document defines the service boundary between Rust capability contracts and
the Python records owned by `mi_contracts`. The boundary is JSON, not Rust or
Python source code. It is intended for file exchange, HTTP, and queues, with an
explicit adapter on each side.

The canonical transport-neutral envelope is
[`schemas/media-intelligence/v1/operation-envelope.schema.json`](../schemas/media-intelligence/v1/operation-envelope.schema.json).
Its schema identifier is
`urn:moenarch:interoperability:media-intelligence:operation-envelope:1`.

## Ownership boundary

| Contract | Owner | Compatibility authority |
| --- | --- | --- |
| v1 operation envelope, diagnostics, normalized artifact references, and error payload | This repository's interoperability schema until a separately reviewed foundation release moves it | `schemas/media-intelligence/v1/operation-envelope.schema.json` |
| Rust capability request and result payloads | The Rust capability crate that defines the operation | The versioned JSON Schema emitted or checked in by that capability |
| Python business and service records, including persistence and orchestration records | `mi_contracts` in `moritzbrantner/media-intelligence` | The versioned schemas and compatibility policy shipped by `mi_contracts` |
| File naming, HTTP routes/authentication, queue names, retry policy, and dead-letter policy | The deploying media-intelligence service | Its service configuration and adapter tests |
| Truth Engine application contracts | `backend-contracts` | The independent Truth Engine package |

`runtime-core` and `jobs-core` remain domain-neutral. They do not absorb Python
business records, service persistence models, prompts, provider configuration,
or orchestration state. `backend-contracts` is a packaging reference only; it
does not own multimodal runtime contracts and is not a dependency of this
exchange.

## Selected exchanges

All selected exchanges use one v1 envelope. `payloadSchema` identifies the
owner and exact version of the embedded payload; the envelope does not make the
payload an interoperability-owned record.

| Exchange | Envelope `messageType` | Payload owner | Required compatibility rule |
| --- | --- | --- | --- |
| Invoke a Rust capability | `request` | The selected Rust capability's request schema | The adapter accepts only a supported `payloadSchema` identifier and maps `payload` to `SurfaceRequest.input`; `operationId` maps to `SurfaceRequest.operation`. |
| Return a successful capability result | `result` | The selected Rust capability's result schema | The adapter maps the typed value to `payload`, diagnostics to `diagnostics`, and normalized `jobs-core::ArtifactRef` values to `artifacts`. |
| Return a capability failure | `error` | The v1 envelope's `surfaceError` definition | The adapter preserves the stable `SurfaceError` fields `code`, `message`, `operation`, and `details`; `payloadSchema` is fixed to `urn:moenarch:runtime:surface-error:1`. |

An adapter may start from a `SurfaceResponse`, an `OperationResult<T>`, or a
typed crate result. It must emit the selected result payload exactly once. For
example, when a `SurfaceResponse.value` contains an `OperationResult<T>`, the
adapter unwraps its `value` into `payload` and moves its diagnostics and
artifacts to the envelope instead of nesting a second result envelope.

Python adapters may construct Rust request payloads from `mi_contracts` records
and may construct `mi_contracts` records from Rust results. Those conversions
are explicit application code. A Python record is never redefined as a Rust
runtime DTO merely because an adapter reads it.

## Transport mappings

The canonical JSON object is identical across transports:

| Transport | Mapping |
| --- | --- |
| File | One UTF-8 JSON envelope per file. Writers publish atomically; readers validate the envelope, then the schema named by `payloadSchema`, before moving the file to processed or rejected storage. File names are not contract identifiers. |
| HTTP | `POST` the request envelope as `application/json`. A successful response body is a result envelope. A capability failure body is an error envelope; HTTP status remains transport metadata. Authentication, tracing headers, and route versions do not enter the envelope. |
| Queue | The message body is exactly one envelope. Broker keys or attributes may duplicate `operationId`, `messageId`, and `correlationId` for routing, but consumers trust the validated body. Redelivery reuses the original `messageId`; broker delivery ids are not contract ids. |

`messageId` identifies one immutable envelope. `correlationId` groups its
request, result, and error. A producer must not reuse a `messageId` after
changing any body field. Queue consumers use `messageId` for deduplication and
acknowledge only after both envelope and payload validation have succeeded.

## Compatibility rules

Version `1` follows these rules:

- Consumers reject an unsupported `schemaVersion` before inspecting `payload`.
- Required fields, field meanings, `messageType` values, and the meaning of an
  existing `operationId` do not change within v1.
- Producers may add optional envelope fields. V1 consumers must ignore unknown
  envelope fields after validating the required fields.
- A breaking envelope change uses a new directory, schema identifier, and
  integer `schemaVersion`. It is not selected by content negotiation alone.
- Payload compatibility is independent. The producer supplies the exact
  versioned `payloadSchema` identifier, and the receiving adapter validates that
  schema before conversion. A breaking payload change receives a new schema
  identifier; an adapter opts in explicitly.
- New Rust operations use new `operationId` values. Reusing an operation id for
  incompatible semantics is forbidden even if the JSON shape happens to match.
- Diagnostic codes and artifact metadata may grow additively. Consumers must
  not treat an unknown diagnostic code or metadata key as an envelope failure.
- An adapter supports an explicit allowlist of `(operationId, payloadSchema)`
  pairs. It must fail closed on an unknown pair rather than guessing from payload
  fields.

These rules allow Rust and Python releases to move independently. A
media-intelligence clean checkout needs a copied or downloaded immutable schema
artifact and its Python dependencies; it never needs a Rust checkout, Cargo
path dependency, generated Rust library, or sibling repository path. Local
co-development may point a test configuration at this schema file, but that path
must not be committed as a production dependency.

## Contract smoke

The checked-in fixtures cover a request, its result, and a correlated error:

- `tests/fixtures/media-intelligence/v1/text-statistics.request.json`
- `tests/fixtures/media-intelligence/v1/text-statistics.result.json`
- `tests/fixtures/media-intelligence/v1/text-statistics.error.json`

Run the envelope-schema and pair-compatibility smoke from the repository root.
The command requires Python `jsonschema` 4.x.

```bash
python3 - <<'PY'
import json
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

schema_path = Path("schemas/media-intelligence/v1/operation-envelope.schema.json")
fixture_root = Path("tests/fixtures/media-intelligence/v1")
schema = json.loads(schema_path.read_text())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema, format_checker=FormatChecker())

messages = {}
for fixture_path in sorted(fixture_root.glob("*.json")):
    message = json.loads(fixture_path.read_text())
    validator.validate(message)
    messages[message["messageType"]] = message

request = messages["request"]
for message_type in ("result", "error"):
    reply = messages[message_type]
    assert reply["operationId"] == request["operationId"]
    assert reply["correlationId"] == request["correlationId"]
    assert reply["causationId"] == request["messageId"]
assert messages["error"]["payload"]["operation"] == request["operationId"]

print("validated media-intelligence v1 envelope fixtures")
PY
```

This smoke validates the shared envelope. Each media-intelligence adapter test
must additionally load the schema named by `payloadSchema`, validate `payload`,
run the explicit conversion, and round-trip the owned type. Rust capability
repositories test their own payload schemas; `media-intelligence` tests the
`mi_contracts` side and the allowlisted adapter pairs.
