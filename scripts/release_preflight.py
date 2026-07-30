#!/usr/bin/env python3
"""Run non-secret release validation and print order; never publish."""

from __future__ import annotations

import argparse
from pathlib import Path

from check_release_plan import main as check_main


if __name__ == "__main__":
    raise SystemExit(check_main())
