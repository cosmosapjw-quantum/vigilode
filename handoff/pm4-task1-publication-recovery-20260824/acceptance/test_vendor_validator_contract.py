from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest


def load_candidate_module():
    root = os.environ.get("PM4_R5_DIR")
    if not root:
        raise AssertionError("PM4_R5_DIR must point to the candidate R5 directory")
    path = Path(root) / "validate_vendor_source.py"
    if not path.is_file():
        raise AssertionError(f"candidate helper missing: {path}")
    spec = importlib.util.spec_from_file_location("candidate_vendor_validator", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def make_vendor(root: Path, count: int, *, include_faer: bool = True) -> Path:
    vendor = root / "vendor"
    vendor.mkdir()
    names = [f"crate-{index:04d}-1.0.0" for index in range(count)]
    if include_faer:
        names[0] = "faer-0.24.4"
    for name in names:
        crate = vendor / name
        crate.mkdir()
        (crate / ".cargo-checksum.json").write_text(
            json.dumps({"files": {}, "package": "0"}, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    return vendor


class VendorValidatorContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.candidate = load_candidate_module()

    def validate(self, vendor: Path):
        result = self.candidate.validate_vendor_source(
            vendor,
            required_packages=("faer-0.24.4",),
        )
        self.assertEqual(result["schema"], "vigilode-cargo-directory-source-validation-v1")
        self.assertFalse(result["exact_package_count_enforced"])
        return result

    def test_262_valid_packages_are_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.validate(make_vendor(Path(tmp), 262))
            self.assertEqual(result["package_directory_count"], 262)
            self.assertEqual(result["checksum_record_count"], 262)

    def test_308_valid_packages_are_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = self.validate(make_vendor(Path(tmp), 308))
            self.assertEqual(result["package_directory_count"], 308)
            self.assertEqual(result["checksum_record_count"], 308)

    def test_missing_checksum_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            vendor = make_vendor(Path(tmp), 8)
            (vendor / "crate-0003-1.0.0" / ".cargo-checksum.json").unlink()
            with self.assertRaises(Exception):
                self.validate(vendor)

    def test_missing_required_crate_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            vendor = make_vendor(Path(tmp), 8, include_faer=False)
            with self.assertRaises(Exception):
                self.validate(vendor)

    def test_output_is_deterministic(self):
        with tempfile.TemporaryDirectory() as tmp:
            vendor = make_vendor(Path(tmp), 8)
            self.assertEqual(self.validate(vendor), self.validate(vendor))


if __name__ == "__main__":
    unittest.main()
