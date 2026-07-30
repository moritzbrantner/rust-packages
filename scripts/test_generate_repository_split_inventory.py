#!/usr/bin/env python3
"""Tests that reviewed ownership remains authority over generated projections."""

from __future__ import annotations

import copy
import unittest

from generate_repository_split_inventory import destination_markdown, generate


class RepositorySplitInventoryTests(unittest.TestCase):
    def test_live_reviewed_authority_matches_manifests(self) -> None:
        authority, _, _, errors = generate()
        self.assertEqual(errors, [])
        self.assertEqual(len(authority["packages"]), 520)

    def test_projection_uses_package_specific_reviewed_decision(self) -> None:
        authority, _, _, errors = generate()
        self.assertEqual(errors, [])
        changed = copy.deepcopy(authority)
        package = next(
            record
            for record in changed["packages"]
            if record["id"] == "cargo:moenarch-video-analysis"
        )
        package["target_repository"] = "nlp-stack"
        markdown = destination_markdown(changed)
        row = next(
            line
            for line in markdown.splitlines()
            if "| `moenarch-video-analysis` |" in line
        )
        self.assertIn("`moritzbrantner/nlp-stack`", row)


if __name__ == "__main__":
    unittest.main(verbosity=2)
