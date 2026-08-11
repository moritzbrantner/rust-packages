#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import io
import subprocess
import tempfile
from pathlib import Path
from typing import Callable

from mi_contracts_model_smoke import (
    EXPECTED_DOWNSTREAM_COMMIT,
    parse_arguments,
    validate_checkout_provenance,
)


CANONICAL_ORIGIN = "https://github.com/moritzbrantner/media-intelligence.git"
SPOOFED_ORIGIN = (
    "https://github.com.lookalike.example/moritzbrantner/media-intelligence.git"
)


def git(*arguments: str) -> None:
    subprocess.run(["git", *arguments], check=True, capture_output=True, text=True)


def clone_scenario(source: Path, destination: Path, origin: str = CANONICAL_ORIGIN) -> Path:
    git("clone", "--quiet", "--no-local", str(source), str(destination))
    git("-C", str(destination), "remote", "set-url", "origin", origin)
    return destination


def expect_value_error(
    label: str, expected_message: str, action: Callable[[], None]
) -> str:
    try:
        action()
    except ValueError as error:
        if expected_message not in str(error):
            raise AssertionError(f"{label} failed for the wrong reason: {error}") from error
        return label
    raise AssertionError(f"provenance mutation unexpectedly passed: {label}")


def validate_sensitivity(source: Path) -> list[str]:
    source = source.resolve()
    if subprocess.run(
        ["git", "-C", str(source), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip() != EXPECTED_DOWNSTREAM_COMMIT:
        raise ValueError(f"sensitivity source must be pinned to {EXPECTED_DOWNSTREAM_COMMIT}")

    rejected: list[str] = []
    with tempfile.TemporaryDirectory(prefix="mi-contracts-provenance-") as directory:
        root = Path(directory)

        canonical = clone_scenario(source, root / "canonical")
        validate_checkout_provenance(canonical)

        spoofed = clone_scenario(source, root / "spoofed", SPOOFED_ORIGIN)
        rejected.append(
            expect_value_error(
                "spoofed origin",
                f"unexpected downstream origin: {SPOOFED_ORIGIN}",
                lambda: validate_checkout_provenance(spoofed),
            )
        )

        alternate = clone_scenario(source, root / "alternate")
        git("-C", str(alternate), "config", "user.name", "Contract Smoke")
        git("-C", str(alternate), "config", "user.email", "contract-smoke@example.invalid")
        git("-C", str(alternate), "commit", "--quiet", "--allow-empty", "-m", "alternate")
        rejected.append(
            expect_value_error(
                "alternate HEAD",
                f"downstream checkout must be pinned to {EXPECTED_DOWNSTREAM_COMMIT}",
                lambda: validate_checkout_provenance(alternate),
            )
        )

        dirty = clone_scenario(source, root / "dirty")
        events_source = dirty / "contracts/src/mi_contracts/events.py"
        events_source.write_text(
            events_source.read_text(encoding="utf-8") + "\n# dirty sensitivity\n",
            encoding="utf-8",
        )
        rejected.append(
            expect_value_error(
                "dirty imported source",
                "downstream checkout must be clean (tracked and untracked): "
                "contracts/src/mi_contracts/events.py",
                lambda: validate_checkout_provenance(dirty),
            )
        )

        ignored = clone_scenario(source, root / "ignored")
        bytecode = ignored / "contracts/src/mi_contracts/__pycache__/events.pyc"
        bytecode.parent.mkdir()
        bytecode.write_bytes(b"ignored sensitivity")
        rejected.append(
            expect_value_error(
                "ignored imported source",
                "downstream imported source must not contain ignored files: "
                "contracts/src/mi_contracts/__pycache__/events.pyc",
                lambda: validate_checkout_provenance(ignored),
            )
        )

    error_output = io.StringIO()
    try:
        with contextlib.redirect_stderr(error_output):
            parse_arguments(
                [
                    "--checkout",
                    str(source),
                    "--expected-commit",
                    "0000000000000000000000000000000000000000",
                ]
            )
    except SystemExit as error:
        if error.code != 2 or "unrecognized arguments: --expected-commit" not in error_output.getvalue():
            raise AssertionError("commit override was rejected for the wrong reason") from error
        rejected.append("commit override argument")
    else:
        raise AssertionError("caller-overridable commit pin unexpectedly remains")
    return rejected


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(
        description="Prove pinned mi_contracts provenance rejects spoof, override, and dirt."
    )
    parser.add_argument("--checkout", type=Path, required=True)
    arguments = parser.parse_args()
    rejected = validate_sensitivity(arguments.checkout)
    print(f"validated canonical provenance and {len(rejected)} rejection scenarios")


if __name__ == "__main__":
    main()
