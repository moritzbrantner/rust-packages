#!/usr/bin/env python3
from __future__ import annotations

import copy
import unittest

from check_spatial_reconciliation import RECONCILIATION_PATH, validate
from repository_split import OWNERSHIP_PATH, load_json


class SpatialReconciliationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.authority = load_json(OWNERSHIP_PATH)
        cls.reconciliation = load_json(RECONCILIATION_PATH)

    def test_current_reconciliation_covers_every_retired_spatial_package(self) -> None:
        self.assertEqual(validate(self.authority, self.reconciliation), [])

    def test_missing_family_fails_closed(self) -> None:
        reconciliation = copy.deepcopy(self.reconciliation)
        reconciliation["families"] = reconciliation["families"][1:]
        errors = validate(self.authority, reconciliation)
        self.assertTrue(any("without a current decision" in error for error in errors), errors)

    def test_duplicate_family_fails_closed(self) -> None:
        reconciliation = copy.deepcopy(self.reconciliation)
        reconciliation["families"].append(copy.deepcopy(reconciliation["families"][0]))
        errors = validate(self.authority, reconciliation)
        self.assertTrue(any("duplicate family" in error for error in errors), errors)

    def test_retained_family_must_stay_with_rust_packages(self) -> None:
        reconciliation = copy.deepcopy(self.reconciliation)
        retained = next(
            item
            for item in reconciliation["families"]
            if item["decision"] == "retained-temporarily"
        )
        retained["currentAuthorityRepository"] = "moritzbrantner/video-to-3d"
        errors = validate(self.authority, reconciliation)
        self.assertTrue(any("retained family must remain" in error for error in errors), errors)

    def test_canonical_destination_needs_real_replacement_surface(self) -> None:
        reconciliation = copy.deepcopy(self.reconciliation)
        canonical = next(
            item
            for item in reconciliation["families"]
            if item["decision"] == "canonical-destination"
        )
        canonical["replacementSurfaces"] = []
        errors = validate(self.authority, reconciliation)
        self.assertTrue(any("needs a proven replacement surface" in error for error in errors), errors)

    def test_retired_spatial_analysis_cannot_reappear_as_authority(self) -> None:
        reconciliation = copy.deepcopy(self.reconciliation)
        reconciliation["families"][0]["currentAuthorityRepository"] = (
            "moritzbrantner/spatial-analysis"
        )
        errors = validate(self.authority, reconciliation)
        self.assertTrue(any("unsupported current authority" in error for error in errors), errors)


if __name__ == "__main__":
    unittest.main()
