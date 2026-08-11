#!/usr/bin/env python3
from __future__ import annotations

import copy
import json
import re
import shutil
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any, Callable
from urllib.parse import unquote, urlparse

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
EVENT_FIXTURE_NAME = "text-statistics.event-envelope.json"
EVENT_FIXTURE_PATH = FIXTURE_ROOT / EVENT_FIXTURE_NAME
SELECTED_EXCHANGE = "text-statistics"
MESSAGE_TYPES = ("request", "result", "error")
OPERATION_FIXTURE_PATTERN = re.compile(
    r"^(?P<exchange>[a-z0-9-]+)\.(?P<message_type>request|result|error)\.json$"
)
FIXTURE_URI_AUTHORITY = "media-intelligence-v1"

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


def discover_operation_fixture_paths(
    fixture_root: Path = FIXTURE_ROOT,
) -> dict[str, Path]:
    discovered: dict[str, Path] = {}
    unknown: list[str] = []
    for path in sorted(fixture_root.rglob("*.json")):
        if path.resolve() == (fixture_root / EVENT_FIXTURE_NAME).resolve():
            continue
        match = OPERATION_FIXTURE_PATTERN.fullmatch(path.name)
        if match is None or match.group("exchange") != SELECTED_EXCHANGE:
            unknown.append(str(path.relative_to(fixture_root)))
            continue
        message_type = match.group("message_type")
        if message_type in discovered:
            unknown.append(str(path.relative_to(fixture_root)))
            continue
        discovered[message_type] = path

    if unknown:
        raise ValueError(
            "unknown or duplicate operation fixture files: " + ", ".join(unknown)
        )
    missing = sorted(set(MESSAGE_TYPES) - discovered.keys())
    if missing:
        raise ValueError("missing operation fixture files: " + ", ".join(missing))
    return discovered


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


def resolve_fixture_pointer(payload_location: str) -> Path:
    parsed = urlparse(payload_location)
    if parsed.scheme != "fixture":
        raise ValueError(f"unauthorized fixture scheme: {parsed.scheme or '<missing>'}")
    if parsed.netloc != FIXTURE_URI_AUTHORITY:
        raise ValueError(f"unauthorized fixture authority: {parsed.netloc or '<missing>'}")
    if parsed.query or parsed.fragment:
        raise ValueError("fixture pointer must not contain a query or fragment")

    relative = Path(unquote(parsed.path).lstrip("/"))
    candidate = (FIXTURE_ROOT / relative).resolve()
    approved_root = FIXTURE_ROOT.resolve()
    try:
        candidate.relative_to(approved_root)
    except ValueError as error:
        raise ValueError("fixture pointer escapes approved fixture root") from error
    if not candidate.is_file():
        raise ValueError(f"fixture pointer does not exist: {relative}")

    approved_targets = set(discover_operation_fixture_paths().values())
    if candidate not in {path.resolve() for path in approved_targets}:
        raise ValueError(f"fixture pointer target is not an approved operation fixture: {relative}")
    return candidate


def validate_event_pointer(
    event: dict[str, Any], selected_request: dict[str, Any]
) -> None:
    EVENT_VALIDATOR.validate(event)
    if not event["payload_location"]:
        raise ValueError("event payload location is required by the pointer adapter")
    pointed_path = resolve_fixture_pointer(event["payload_location"])
    pointed_message = load_json(pointed_path)
    validate_message(pointed_message)

    if event["schema_version"] != "v1" or event["metadata"]["schema_version"] != "v1":
        raise ValueError("EventEnvelopeV1 version does not map to operation envelope v1")
    if pointed_message["schemaVersion"] != 1:
        raise ValueError("operation envelope version does not map to EventEnvelopeV1")
    if event["event_id"] != pointed_message["messageId"]:
        raise ValueError("event id must equal pointed-to message id")
    if event["event_type"] != f"rust.operation.{pointed_message['messageType']}":
        raise ValueError("event type does not identify the pointed-to message type")
    if event["timestamp"] != pointed_message["occurredAt"]:
        raise ValueError("event timestamp must equal operation occurrence time")
    if event["metadata"]["source_type"] != "rust-capability-operation-envelope":
        raise ValueError("event source type does not identify the pointer adapter")
    if pointed_message != selected_request:
        raise ValueError("event pointer does not resolve to the selected request fixture")


def validate_exchange(
    messages: list[dict[str, Any]], event: dict[str, Any] | None = None
) -> None:
    for message in messages:
        validate_message(message)
    validate_relationships(messages)
    validate_source_truth(messages)
    if event is not None:
        validate_event_pointer(event, by_type(messages)["request"])


def expect_rejected(
    label: str,
    expected_exception: type[Exception],
    expected_message: str,
    action: Callable[[], None],
) -> str:
    try:
        action()
    except expected_exception as error:
        messages = [str(error)]
        if isinstance(error, ValidationError):
            pending = list(error.context)
            while pending:
                context = pending.pop()
                messages.append(context.message)
                pending.extend(context.context)
        if not any(expected_message in message for message in messages):
            raise AssertionError(
                f"{label} failed for the wrong reason: {error}"
            ) from error
        return label
    raise AssertionError(f"sensitivity mutation unexpectedly passed: {label}")


def sensitivity_checks(
    messages: list[dict[str, Any]], event: dict[str, Any]
) -> list[str]:
    typed = by_type(messages)
    rejected = []

    unknown_pair = copy.deepcopy(messages)
    by_type(unknown_pair)["request"]["payloadSchema"] = RESULT_SCHEMA_ID
    rejected.append(
        expect_rejected(
            "unknown allowlist tuple",
            ValueError,
            "exchange is not allowlisted",
            lambda: validate_exchange(unknown_pair),
        )
    )

    bad_correlation = copy.deepcopy(messages)
    by_type(bad_correlation)["result"]["correlationId"] = "corr-wrong"
    rejected.append(
        expect_rejected(
            "mismatched correlation",
            ValueError,
            "result correlation mismatch",
            lambda: validate_exchange(bad_correlation),
        )
    )

    bad_causation = copy.deepcopy(messages)
    by_type(bad_causation)["error"]["causationId"] = "msg-wrong"
    rejected.append(
        expect_rejected(
            "mismatched causation",
            ValueError,
            "error causation mismatch",
            lambda: validate_exchange(bad_causation),
        )
    )

    wrong_error_schema = copy.deepcopy(messages)
    by_type(wrong_error_schema)["error"]["payloadSchema"] = REQUEST_SCHEMA_ID
    rejected.append(
        expect_rejected(
            "wrong error schema",
            ValueError,
            "exchange is not allowlisted",
            lambda: validate_exchange(wrong_error_schema),
        )
    )

    missing_required = copy.deepcopy(messages)
    del by_type(missing_required)["request"]["messageId"]
    rejected.append(
        expect_rejected(
            "missing required field",
            ValidationError,
            "'messageId' is a required property",
            lambda: validate_exchange(missing_required),
        )
    )

    malformed_payload = copy.deepcopy(messages)
    by_type(malformed_payload)["request"]["payload"]["text"] = 42
    rejected.append(
        expect_rejected(
            "malformed payload",
            ValidationError,
            "is not of type 'string'",
            lambda: validate_exchange(malformed_payload),
        )
    )

    duplicate_type = copy.deepcopy(messages)
    duplicate_type.append(copy.deepcopy(typed["request"]))
    rejected.append(
        expect_rejected(
            "duplicate message type",
            ValueError,
            "expected one request/result/error",
            lambda: validate_exchange(duplicate_type),
        )
    )

    rejected.append(
        expect_rejected(
            "unresolved payload schema",
            ValueError,
            "unresolved payload schema",
            lambda: resolve_payload_schema("urn:moenarch:unknown:payload:1"),
        )
    )

    invented_error = copy.deepcopy(messages)
    by_type(invented_error)["error"]["payload"]["operation"] = "text.statistics"
    rejected.append(
        expect_rejected(
            "non-source error fixture",
            ValueError,
            "malformed text.statistics error is not source true",
            lambda: validate_exchange(invented_error),
        )
    )

    missing_pointer = copy.deepcopy(event)
    missing_pointer["payload_location"] = None
    rejected.append(
        expect_rejected(
            "missing event pointer",
            ValueError,
            "event payload location is required",
            lambda: validate_exchange(messages, missing_pointer),
        )
    )

    unauthorized_pointer = copy.deepcopy(event)
    unauthorized_pointer["payload_location"] = (
        "fixture://other-authority/text-statistics.request.json"
    )
    rejected.append(
        expect_rejected(
            "unauthorized event pointer",
            ValueError,
            "unauthorized fixture authority",
            lambda: validate_exchange(messages, unauthorized_pointer),
        )
    )

    escaping_pointer = copy.deepcopy(event)
    escaping_pointer["payload_location"] = (
        f"fixture://{FIXTURE_URI_AUTHORITY}/../text-statistics.request.json"
    )
    rejected.append(
        expect_rejected(
            "escaping event pointer",
            ValueError,
            "escapes approved fixture root",
            lambda: validate_exchange(messages, escaping_pointer),
        )
    )

    nonexistent_pointer = copy.deepcopy(event)
    nonexistent_pointer["payload_location"] = (
        f"fixture://{FIXTURE_URI_AUTHORITY}/not-found.request.json"
    )
    rejected.append(
        expect_rejected(
            "nonexistent event pointer",
            ValueError,
            "fixture pointer does not exist",
            lambda: validate_exchange(messages, nonexistent_pointer),
        )
    )

    mismatched_pointer = copy.deepcopy(event)
    mismatched_pointer["payload_location"] = (
        f"fixture://{FIXTURE_URI_AUTHORITY}/text-statistics.result.json"
    )
    mismatched_pointer["event_id"] = typed["result"]["messageId"]
    mismatched_pointer["event_type"] = "rust.operation.result"
    mismatched_pointer["timestamp"] = typed["result"]["occurredAt"]
    rejected.append(
        expect_rejected(
            "mismatched event pointer",
            ValueError,
            "does not resolve to the selected request fixture",
            lambda: validate_exchange(messages, mismatched_pointer),
        )
    )
    return rejected


def fixture_discovery_sensitivity() -> str:
    with tempfile.TemporaryDirectory(prefix="media-intelligence-fixtures-") as directory:
        temporary_root = Path(directory)
        for fixture in FIXTURE_ROOT.glob("*.json"):
            shutil.copy2(fixture, temporary_root / fixture.name)
        shutil.copy2(
            FIXTURE_ROOT / "text-statistics.request.json",
            temporary_root / "unexpected.request.json",
        )
        return expect_rejected(
            "extra on-disk operation fixture",
            ValueError,
            "unknown or duplicate operation fixture files: unexpected.request.json",
            lambda: discover_operation_fixture_paths(temporary_root),
        )


def exception_narrowing_sensitivity() -> str:
    try:
        expect_rejected(
            "unrelated exception",
            ValueError,
            "expected validation failure",
            lambda: {}["unexpected-bug"],
        )
    except KeyError as error:
        if error.args != ("unexpected-bug",):
            raise AssertionError("unexpected KeyError payload") from error
        return "unrelated exception propagation"
    raise AssertionError("unrelated KeyError was incorrectly counted as a rejection")


def main() -> None:
    message_paths = discover_operation_fixture_paths()
    messages = [load_json(message_paths[message_type]) for message_type in MESSAGE_TYPES]
    event = load_json(EVENT_FIXTURE_PATH)
    validate_exchange(messages, event)
    rejected = sensitivity_checks(messages, event)
    rejected.append(fixture_discovery_sensitivity())
    rejected.append(exception_narrowing_sensitivity())
    print(
        "validated discovered media-intelligence v1 schemas, payloads, loaded pointer, "
        f"and {len(rejected)} narrowly asserted sensitivity mutations"
    )


if __name__ == "__main__":
    main()
