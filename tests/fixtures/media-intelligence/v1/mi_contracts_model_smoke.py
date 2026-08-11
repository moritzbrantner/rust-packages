#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence


EXPECTED_DOWNSTREAM_COMMIT = "e5b49cdd32acbfdaca057dc05d12412899f3129d"
CANONICAL_ORIGIN_PATTERNS = (
    re.compile(
        r"https://github\.com/moritzbrantner/media-intelligence(?:\.git)?/?"
    ),
    re.compile(
        r"git@github\.com:moritzbrantner/media-intelligence(?:\.git)?/?"
    ),
    re.compile(
        r"ssh://git@github\.com/moritzbrantner/media-intelligence(?:\.git)?/?"
    ),
)
EVENT_FIXTURE_PATH = Path(__file__).with_name("text-statistics.event-envelope.json")
IMPORTED_SOURCE_PATH = Path("contracts/src")


def git_output(checkout: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(checkout), *arguments],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.rstrip()


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def is_canonical_origin(origin: str) -> bool:
    normalized = origin.strip()
    return any(pattern.fullmatch(normalized) for pattern in CANONICAL_ORIGIN_PATTERNS)


def changed_paths(status: str) -> str:
    return ", ".join(line[3:] for line in status.splitlines()[:5])


def validate_checkout_provenance(checkout: Path) -> Path:
    checkout = checkout.resolve()
    actual_commit = git_output(checkout, "rev-parse", "HEAD")
    if actual_commit != EXPECTED_DOWNSTREAM_COMMIT:
        raise ValueError(
            "downstream checkout must be pinned to "
            f"{EXPECTED_DOWNSTREAM_COMMIT}, got {actual_commit}"
        )

    origin = git_output(checkout, "remote", "get-url", "origin")
    if not is_canonical_origin(origin):
        raise ValueError(f"unexpected downstream origin: {origin}")

    ordinary_status = git_output(
        checkout, "status", "--porcelain=v1", "--untracked-files=all"
    )
    if ordinary_status:
        raise ValueError(
            "downstream checkout must be clean (tracked and untracked): "
            + changed_paths(ordinary_status)
        )

    imported_ignored_status = git_output(
        checkout,
        "status",
        "--porcelain=v1",
        "--ignored",
        "--untracked-files=all",
        "--",
        str(IMPORTED_SOURCE_PATH),
    )
    ignored_source = "\n".join(
        line for line in imported_ignored_status.splitlines() if line.startswith("!! ")
    )
    if ignored_source:
        raise ValueError(
            "downstream imported source must not contain ignored files: "
            + changed_paths(ignored_source)
        )

    contracts_source = checkout / IMPORTED_SOURCE_PATH
    if not (contracts_source / "mi_contracts/events.py").is_file():
        raise ValueError(f"mi_contracts.events is absent from {contracts_source}")
    return contracts_source


def validate_with_downstream_model(checkout: Path) -> None:
    checkout = checkout.resolve()
    contracts_source = validate_checkout_provenance(checkout)
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(contracts_source))
    from mi_contracts.events import EventEnvelopeV1

    fixture = load_json(EVENT_FIXTURE_PATH)
    model = EventEnvelopeV1.model_validate(fixture)
    if model.model_dump(mode="json") != fixture:
        raise ValueError("EventEnvelopeV1 model round-trip changed the pointer fixture")
    if model.payload_location != fixture["payload_location"]:
        raise ValueError("EventEnvelopeV1 did not preserve payload_location")
    validate_checkout_provenance(checkout)


def parse_arguments(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate the pointer fixture with the immutable pinned mi_contracts model."
    )
    parser.add_argument("--checkout", type=Path, required=True)
    return parser.parse_args(arguments)


def main() -> None:
    arguments = parse_arguments()
    validate_with_downstream_model(arguments.checkout)
    print(
        "validated pointer fixture with mi_contracts.events.EventEnvelopeV1 at "
        f"{EXPECTED_DOWNSTREAM_COMMIT} from canonical GitHub origin and clean source"
    )


if __name__ == "__main__":
    main()
