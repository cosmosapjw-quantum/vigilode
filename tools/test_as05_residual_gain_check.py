#!/usr/bin/env python3
"""Unit and mutation tests for tools/as05_residual_gain_check.py."""

import copy
import json
import sys
import tempfile
import unittest
from fractions import Fraction as F
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from as05_residual_gain_check import (
    PINNED_HASHES,
    REPO_ROOT,
    check_authority_types,
    compute_sha256,
    hex_to_f64,
    hex_to_fraction,
    solve_fraction_system,
    verify_runs_file,
)


class TestResidualGainChecker(unittest.TestCase):
    def test_sha256_pinned_hashes(self):
        """Verifies that all 4 re-audit reference files are byte-identical and match SHA-256 pins."""
        for rel_path, expected_hash in PINNED_HASHES.items():
            full_path = REPO_ROOT / rel_path
            self.assertTrue(full_path.is_file(), f"Missing file: {rel_path}")
            actual_hash = compute_sha256(full_path)
            self.assertEqual(
                actual_hash,
                expected_hash,
                f"Hash mismatch on {rel_path}: expected {expected_hash}, got {actual_hash}",
            )

    def test_authority_types_pass(self):
        """Inspects residual_gain.rs to ensure authority types have no Deserialize and no public fields."""
        res = check_authority_types(REPO_ROOT / "crates/rodas5p-core/src/residual_gain.rs")
        self.assertEqual(res["status"], "PASS")

    def test_authority_types_mutation_deserialize(self):
        """Negative control: mutating an authority type to derive Deserialize must be caught."""
        with tempfile.NamedTemporaryFile("w+", suffix=".rs") as tf:
            tf.write("""
#[derive(Clone, Debug, Deserialize)]
pub struct VerifiedCorrection {
    field: f64,
}
""")
            tf.flush()
            res = check_authority_types(Path(tf.name))
            self.assertEqual(res["status"], "FAIL")
            self.assertIn("Deserialize", res["reason"])

    def test_authority_types_mutation_public_field(self):
        """Negative control: mutating an authority type to expose a public field must be caught."""
        with tempfile.NamedTemporaryFile("w+", suffix=".rs") as tf:
            tf.write("""
#[derive(Clone, Debug)]
pub struct LinearTargetId {
    pub inner: u64,
}
""")
            tf.flush()
            res = check_authority_types(Path(tf.name))
            self.assertEqual(res["status"], "FAIL")
            self.assertIn("public field", res["reason"])

    def test_solve_fraction_system(self):
        """Tests exact Fraction Gaussian elimination on 2x2 and 3x3 linear systems."""
        # 2x2 system: [[2, 1], [1, 3]] x = [4, 7] -> x = [1, 2]
        A = [[F(2), F(1)], [F(1), F(3)]]
        b = [F(4), F(7)]
        x = solve_fraction_system(A, b)
        self.assertEqual(x, [F(1), F(2)])

        # Singular system must raise ValueError
        A_sing = [[F(1), F(2)], [F(2), F(4)]]
        with self.assertRaises(ValueError):
            solve_fraction_system(A_sing, [F(1), F(2)])

    def test_hex_conversions(self):
        """Tests bit-level hex conversion to float and Fraction."""
        # 1.0 in binary64 is 0x3ff0000000000000
        h1 = "3ff0000000000000"
        self.assertEqual(hex_to_f64(h1), 1.0)
        self.assertEqual(hex_to_fraction(h1), F(1))

        # -0.5 is 0xbfe0000000000000
        h2 = "bfe0000000000000"
        self.assertEqual(hex_to_f64(h2), -0.5)
        self.assertEqual(hex_to_fraction(h2), F(-1, 2))

    def test_full_check_runs_pass(self):
        """Verifies that the generated RUNS.json produces a PASS verdict on all 307 cases."""
        runs_path = REPO_ROOT / "research/as05_residual_gain_core_20261011/RUNS.json"
        retained_path = REPO_ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json"
        self.assertTrue(runs_path.is_file(), "RUNS.json not found")
        self.assertTrue(retained_path.is_file(), "NATIVE.json not found")

        report = verify_runs_file(runs_path, retained_path)
        self.assertEqual(report["verdict"], "PASS")
        self.assertEqual(report["total_cases"], 307)
        self.assertEqual(report["passed_cases"], 307)
        self.assertEqual(report["failed_cases"], 0)
        self.assertEqual(report["case_failures"], [])
        self.assertEqual(report["authority_check"]["status"], "PASS")

    def test_mutation_tampered_interval_fails(self):
        """Negative control: tampering an enclosure interval to exclude true error must cause FAIL."""
        runs_path = REPO_ROOT / "research/as05_residual_gain_core_20261011/RUNS.json"
        retained_path = REPO_ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json"
        with open(runs_path) as f:
            data = json.load(f)

        # Mutate case 0 weighted correction interval to [0, 0] when true error is non-zero
        data["cases"][0]["weighted_correction_intervals_hex"] = [
            ["0000000000000000", "0000000000000000"],
            ["0000000000000000", "0000000000000000"],
        ]

        with tempfile.NamedTemporaryFile("w+", suffix=".json") as tf:
            json.dump(data, tf)
            tf.flush()
            report = verify_runs_file(Path(tf.name), retained_path)
            self.assertEqual(report["verdict"], "FAIL")
            self.assertGreater(report["failed_cases"], 0)

    def test_mutation_tampered_rejection_class_fails(self):
        """Negative control: tampering a negative control rejection class must cause FAIL."""
        runs_path = REPO_ROOT / "research/as05_residual_gain_core_20261011/RUNS.json"
        retained_path = REPO_ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json"
        with open(runs_path) as f:
            data = json.load(f)

        # Find a metric mismatch negative and change rejection class
        for c in data["cases"]:
            if c.get("category") == "case_c_metric":
                c["rejection_class"] = "stale_epoch"
                break

        with tempfile.NamedTemporaryFile("w+", suffix=".json") as tf:
            json.dump(data, tf)
            tf.flush()
            report = verify_runs_file(Path(tf.name), retained_path)
            self.assertEqual(report["verdict"], "FAIL")
            self.assertGreater(report["failed_cases"], 0)

    def test_mutation_tampered_schema_invalid(self):
        """Negative control: invalid schema must cause INVALID verdict."""
        runs_path = REPO_ROOT / "research/as05_residual_gain_core_20261011/RUNS.json"
        retained_path = REPO_ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json"
        with open(runs_path) as f:
            data = json.load(f)

        data["schema"] = "corrupted-schema-v0"
        with tempfile.NamedTemporaryFile("w+", suffix=".json") as tf:
            json.dump(data, tf)
            tf.flush()
            report = verify_runs_file(Path(tf.name), retained_path)
            self.assertEqual(report["verdict"], "INVALID")

    def test_mutation_tampered_transport_gain_fails(self):
        """Negative control: tampering transported gain bound below true gain must cause FAIL."""
        runs_path = REPO_ROOT / "research/as05_residual_gain_core_20261011/RUNS.json"
        retained_path = REPO_ROOT / "research/reaudit_accuracy_speed_20261010/NATIVE.json"
        with open(runs_path) as f:
            data = json.load(f)

        # Find a transport case and set scaled_residual_gain_inf_upper to 0.001
        for c in data["cases"]:
            if c.get("category") == "case_d_transport":
                c["scaled_residual_gain_inf_upper"] = 0.0001
                break

        with tempfile.NamedTemporaryFile("w+", suffix=".json") as tf:
            json.dump(data, tf)
            tf.flush()
            report = verify_runs_file(Path(tf.name), retained_path)
            self.assertEqual(report["verdict"], "FAIL")
            self.assertGreater(report["failed_cases"], 0)


if __name__ == "__main__":
    unittest.main()

