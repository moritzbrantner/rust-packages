#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


EXPECTED_DOWNSTREAM_COMMIT = "e5b49cdd32acbfdaca057dc05d12412899f3129d"
EXPECTED_ORIGIN_SUFFIX = "moritzbrantner/media-intelligence.git"
EVENT_FIXTURE_PATH = Path(__file__).with_name("text-statistics.event-envelope.json")


def git_output(checkout: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(checkout), *arguments],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_with_downstream_model(checkout: Path, expected_commit: str) -> None:
    checkout = checkout.resolve()
    actual_commit = git_output(checkout, "rev-parse", "HEAD")
    if actual_commit != expected_commit:
        raise ValueError(
            f"downstream checkout must be pinned to {expected_commit}, got {actual_commit}"
        )
    origin = git_output(checkout, "remote", "get-url", "origin")
    if not origin.rstrip("/").endswith(EXPECTED_ORIGIN_SUFFIX):
        raise ValueError(f"unexpected downstream origin: {origin}")

    contracts_source = checkout / "contracts/src"
    if not (contracts_source / "mi_contracts/events.py").is_file():
        raise ValueError(f"mi_contracts.events is absent from {contracts_source}")
    sys.path.insert(0, str(contracts_source))
    from mi_contracts.events import EventEnvelopeV1

    fixture = load_json(EVENT_FIXTURE_PATH)
    model = EventEnvelopeV1.model_validate(fixture)
    if model.model_dump(mode="json") != fixture:
        raise ValueError("EventEnvelopeV1 model round-trip changed the pointer fixture")
    if model.payload_location != fixture["payload_location"]:
        raise ValueError("EventEnvelopeV1 did not preserve payload_location")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate the pointer fixture with the actual pinned mi_contracts model."
    )
    parser.add_argument("--checkout", type=Path, required=True)
    parser.add_argument("--expected-commit", default=EXPECTED_DOWNSTREAM_COMMIT)
    arguments = parser.parse_args()
    validate_with_downstream_model(arguments.checkout, arguments.expected_commit)
    print(
        "validated pointer fixture with mi_contracts.events.EventEnvelopeV1 at "
        f"{arguments.expected_commit}"
    )


if __name__ == "__main__":
    main()
