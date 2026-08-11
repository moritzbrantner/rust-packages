#!/usr/bin/env python3
from __future__ import annotations

import copy
import json
from collections import Counter
from pathlib import Path
from typing import Any, Callable

from jsonschema import Draft202012Validator, FormatChecker
from jsonschema.exceptions import ValidationError


REPOSITORY_ROOT = Path(__file__).resolve().parents[4]
FIXTURE_ROOT = REPOSITORY_ROOT / "tests/fixtures/media-intelligence/v1"
SCHEMA_ROOT = REPOSITORY_ROOT / "schemas/media-intelligence"

ENVELOPE_SCHEMA_PATH = SCHEMA_ROOT / "v1/operation-envelope.schema.json"
PAYLOAD_SCHEMA_PATHS = (
    SCHEMA_ROOT / "v1/payloads/text-statistics-request.schema.json",
    SCHEMA_ROOT / "v1/payloads/text-statistics-surface-response.schema.json",
    SCHEMA_ROOT / "v1/payloads/surface-error.schema.json",
)
EVENT_SCHEMA_PATH = (
    SCHEMA_ROOT / "external/mi-contracts/0.1.0/event-envelope-v1.schema.json"
)
MESSAGE_PATHS = {
    message_type: FIXTURE_ROOT / f"text-statistics.{message_type}.json"
    for message_type in ("request", "result", "error")
}
EVENT_FIXTURE_PATH = FIXTURE_ROOT / "text-statistics.event-envelope.json"

REQUEST_SCHEMA_ID = "urn:moenarch:text-core:text-statistics-request:0.1.0"
RESULT_SCHEMA_ID = (
    "urn:moenarch:text-core:text-statistics-surface-response:0.1.0"
)
ERROR_SCHEMA_ID = "urn:moenarch:runtime-core:surface-error:0.2.0"
ALLOWED_EXCHANGES = {
    ("request", "text.statistics", REQUEST_SCHEMA_ID),
    ("result", "text.statistics", RESULT_SCHEMA_ID),
    ("error", "text.statistics", ERROR_SCHEMA_ID),
}


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def validator(schema: dict[str, Any]) -> Draft202012Validator:
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema, format_checker=FormatChecker())


ENVELOPE_VALIDATOR = validator(load_json(ENVELOPE_SCHEMA_PATH))
PAYLOAD_SCHEMAS = {schema["$id"]: schema for schema in map(load_json, PAYLOAD_SCHEMA_PATHS)}
if len(PAYLOAD_SCHEMAS) != len(PAYLOAD_SCHEMA_PATHS):
    raise AssertionError("payload schema ids must be unique")
PAYLOAD_VALIDATORS = {
    schema_id: validator(schema) for schema_id, schema in PAYLOAD_SCHEMAS.items()
}
EVENT_VALIDATOR = validator(load_json(EVENT_SCHEMA_PATH))


def resolve_payload_schema(schema_id: str) -> Draft202012Validator:
    try:
        return PAYLOAD_VALIDATORS[schema_id]
    except KeyError as error:
        raise ValueError(f"unresolved payload schema: {schema_id}") from error


def validate_message(message: dict[str, Any]) -> None:
    ENVELOPE_VALIDATOR.validate(message)
    exchange = (
        message["messageType"],
        message["operationId"],
        message["payloadSchema"],
    )
    if exchange not in ALLOWED_EXCHANGES:
        raise ValueError(f"exchange is not allowlisted: {exchange}")
    resolve_payload_schema(message["payloadSchema"]).validate(message["payload"])


def by_type(messages: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    counts = Counter(message["messageType"] for message in messages)
    if counts != Counter({"request": 1, "result": 1, "error": 1}):
        raise ValueError(f"expected one request/result/error, got {dict(counts)}")
    return {message["messageType"]: message for message in messages}


def validate_relationships(messages: list[dict[str, Any]]) -> None:
    typed = by_type(messages)
    request = typed["request"]
    for message_type in ("result", "error"):
        reply = typed[message_type]
        if reply["operationId"] != request["operationId"]:
            raise ValueError(f"{message_type} operation mismatch")
        if reply["correlationId"] != request["correlationId"]:
            raise ValueError(f"{message_type} correlation mismatch")
        if reply["causationId"] != request["messageId"]:
            raise ValueError(f"{message_type} causation mismatch")

    surface = typed["result"]["payload"]
    if surface["operation"] != request["operationId"]:
        raise ValueError("SurfaceResponse operation mismatch")
    structured = surface["value"]
    operation_result = structured["result"]
    for field in ("value", "diagnostics", "artifacts"):
        if structured[field] != operation_result[field]:
            raise ValueError(f"structured response projection mismatch: {field}")

    error_operation = typed["error"]["payload"]["operation"]
    if error_operation is not None and error_operation != request["operationId"]:
        raise ValueError("SurfaceError operation mismatch")


def validate_source_truth(messages: list[dict[str, Any]]) -> None:
    typed = by_type(messages)
    expected_error = {
        "code": "invalid_request",
        "message": "invalid request: missing field `text`",
        "operation": None,
        "details": {},
    }
    if typed["error"]["payload"] != expected_error:
        raise ValueError("malformed text.statistics error is not source true")
    expected_value = {
        "byteCount": 19,
        "characterCount": 19,
        "lineCount": 1,
        "sentenceCount": 2,
        "wordCount": 3,
    }
    if typed["result"]["payload"]["value"]["result"]["value"] != expected_value:
        raise ValueError("text.statistics result is not source true")


def validate_event_pointer(event: dict[str, Any], request: dict[str, Any]) -> None:
    EVENT_VALIDATOR.validate(event)
    if event["schema_version"] != "v1" or event["metadata"]["schema_version"] != "v1":
        raise ValueError("EventEnvelopeV1 version does not map to operation envelope v1")
    if request["schemaVersion"] != 1:
        raise ValueError("operation envelope version does not map to EventEnvelopeV1")
    if event["event_id"] != request["messageId"]:
        raise ValueError("event id must equal pointed-to message id")
    if event["event_type"] != f"rust.operation.{request['messageType']}":
        raise ValueError("event type does not identify the pointed-to message type")
    if event["timestamp"] != request["occurredAt"]:
        raise ValueError("event timestamp must equal operation occurrence time")
    if not event["payload_location"]:
        raise ValueError("event payload location is required by the pointer adapter")
    if not event["payload_location"].endswith(MESSAGE_PATHS["request"].name):
        raise ValueError("event payload location does not point to the request fixture")
    if event["metadata"]["source_type"] != "rust-capability-operation-envelope":
        raise ValueError("event source type does not identify the pointer adapter")


def validate_exchange(
    messages: list[dict[str, Any]], event: dict[str, Any] | None = None
) -> None:
    for message in messages:
        validate_message(message)
    validate_relationships(messages)
    validate_source_truth(messages)
    if event is not None:
        validate_event_pointer(event, by_type(messages)["request"])


def expect_rejected(label: str, action: Callable[[], None]) -> str:
    try:
        action()
    except (AssertionError, KeyError, TypeError, ValidationError, ValueError):
        return label
    raise AssertionError(f"sensitivity mutation unexpectedly passed: {label}")


def sensitivity_checks(
    messages: list[dict[str, Any]], event: dict[str, Any]
) -> list[str]:
    typed = by_type(messages)
    rejected = []

    unknown_pair = copy.deepcopy(messages)
    by_type(unknown_pair)["request"]["payloadSchema"] = RESULT_SCHEMA_ID
    rejected.append(expect_rejected("unknown allowlist tuple", lambda: validate_exchange(unknown_pair)))

    bad_correlation = copy.deepcopy(messages)
    by_type(bad_correlation)["result"]["correlationId"] = "corr-wrong"
    rejected.append(expect_rejected("mismatched correlation", lambda: validate_exchange(bad_correlation)))

    bad_causation = copy.deepcopy(messages)
    by_type(bad_causation)["error"]["causationId"] = "msg-wrong"
    rejected.append(expect_rejected("mismatched causation", lambda: validate_exchange(bad_causation)))

    wrong_error_schema = copy.deepcopy(messages)
    by_type(wrong_error_schema)["error"]["payloadSchema"] = REQUEST_SCHEMA_ID
    rejected.append(expect_rejected("wrong error schema", lambda: validate_exchange(wrong_error_schema)))

    missing_required = copy.deepcopy(messages)
    del by_type(missing_required)["request"]["messageId"]
    rejected.append(expect_rejected("missing required field", lambda: validate_exchange(missing_required)))

    malformed_payload = copy.deepcopy(messages)
    by_type(malformed_payload)["request"]["payload"]["text"] = 42
    rejected.append(expect_rejected("malformed payload", lambda: validate_exchange(malformed_payload)))

    duplicate_type = copy.deepcopy(messages)
    duplicate_type.append(copy.deepcopy(typed["request"]))
    rejected.append(expect_rejected("duplicate message type", lambda: validate_exchange(duplicate_type)))

    rejected.append(
        expect_rejected(
            "unresolved payload schema",
            lambda: resolve_payload_schema("urn:moenarch:unknown:payload:1"),
        )
    )

    invented_error = copy.deepcopy(messages)
    by_type(invented_error)["error"]["payload"]["operation"] = "text.statistics"
    rejected.append(expect_rejected("non-source error fixture", lambda: validate_exchange(invented_error)))

    wrong_event = copy.deepcopy(event)
    wrong_event["payload_location"] = None
    rejected.append(
        expect_rejected(
            "missing event pointer",
            lambda: validate_exchange(messages, wrong_event),
        )
    )
    return rejected


def main() -> None:
    messages = [load_json(MESSAGE_PATHS[message_type]) for message_type in ("request", "result", "error")]
    event = load_json(EVENT_FIXTURE_PATH)
    validate_exchange(messages, event)
    rejected = sensitivity_checks(messages, event)
    print(
        "validated media-intelligence v1 schemas, payloads, pointer adapter, "
        f"and {len(rejected)} sensitivity mutations"
    )


if __name__ == "__main__":
    main()
