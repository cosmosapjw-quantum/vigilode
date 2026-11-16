#!/usr/bin/env python3
"""
Unit tests for bench/build_unified_comparison.py
Verifies fixes for BENCH-001, BENCH-002, BENCH-003, and BENCH-004.
"""

import unittest
import math
import tempfile
import json
import os
import sys

# Ensure bench is in python path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
import build_unified_comparison as buc

class TestReportGenerator(unittest.TestCase):

    def test_format_rtol_heading(self):
        """BENCH-002: Test numerical exponent parsing across various formats."""
        test_cases = [
            (0.0001, "10^{-4}"),
            (1e-4, "10^{-4}"),
            (1e-6, "10^{-6}"),
            (1e-06, "10^{-6}"),
            (1e-8, "10^{-8}"),
            (1e-08, "10^{-8}"),
            (1e-5, "10^{-5}"),
            (1e-10, "10^{-10}"),
            (1e-12, "10^{-12}"),
        ]
        for val, expected_sub in test_cases:
            heading = buc.format_rtol_heading(val)
            self.assertIn(expected_sub, heading, f"Failed for {val}: got {heading}")
            self.assertNotIn("10^{-2}", heading if val not in [0.01, 1e-2] else "", f"False 10^-2 for {val}")

    def test_format_allocations(self):
        """BENCH-001: Ensure raw heap_calls and bytes are faithfully reported."""
        # When heap_calls > 0, must NOT say "0 (Zero-Alloc)"
        alloc_data = {"heap_calls": 272, "heap_bytes": 23846}
        s = buc.format_allocations(alloc_data)
        self.assertIn("272", s)
        self.assertIn("23.3 KB", s)
        self.assertNotIn("0 (Zero-Alloc)", s)

        # Exact zero
        alloc_zero = {"heap_calls": 0, "heap_bytes": 0}
        s_zero = buc.format_allocations(alloc_zero)
        self.assertIn("0 calls", s_zero)

        # None / missing
        self.assertEqual(buc.format_allocations(None), "—")

    def test_duplicate_key_detection(self):
        """BENCH-004: Hard-fail on duplicate (problem, arm, rtol) keys."""
        data_with_duplicates = {
            "schema": "vigilode.bench.v1",
            "provenance": {"host_cpu": "Intel Xeon"},
            "rows": [
                {"problem": "robertson", "arm": "vigilode", "rtol": 1e-4, "status": {"kind": "success"}},
                {"problem": "robertson", "arm": "vigilode", "rtol": 1e-4, "status": {"kind": "success"}}
            ]
        }
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(data_with_duplicates, f)
            wp_path = f.name

        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump({"schema": "vigilode.cross_solver_run.v2", "results": []}, f)
            jl_path = f.name

        try:
            with self.assertRaises(ValueError) as ctx:
                buc.load_and_validate_data(wp_path, jl_path, enforce_completeness=False)
            self.assertIn("Duplicate entry", str(ctx.exception))
        finally:
            os.remove(wp_path)
            os.remove(jl_path)

    def test_missing_required_cells_detection(self):
        """BENCH-004: Hard-fail if expected problem/arm/rtol cells are missing."""
        minimal_incomplete_rust = {
            "schema": "vigilode.bench.v1",
            "provenance": {"host_cpu": "Intel Xeon"},
            "rows": [
                {"problem": "robertson", "arm": "vigilode", "rtol": 1e-4, "status": {"kind": "success"}}
            ]
        }
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(minimal_incomplete_rust, f)
            wp_path = f.name

        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump({"schema": "vigilode.cross_solver_run.v2", "results": []}, f)
            jl_path = f.name

        try:
            with self.assertRaises(ValueError) as ctx:
                buc.load_and_validate_data(wp_path, jl_path, enforce_completeness=True)
            self.assertIn("Missing expected cell", str(ctx.exception))
        finally:
            os.remove(wp_path)
            os.remove(jl_path)

    def test_host_provenance_check(self):
        """BENCH-003: Check host compatibility between Rust and Julia runners."""
        rust_meta = {"host_cpu": "Intel(R) Xeon(R) Processor @ 2.10GHz", "host_logical_cpus": 2}
        jl_meta_diff = {"host_cpu": "AMD Ryzen 9 5900X 12-Core Processor", "host_logical_cpus": 24}
        jl_meta_same = {"host_cpu": "Intel(R) Xeon(R) Processor @ 2.10GHz", "host_logical_cpus": 2}

        # Different hosts
        comp_diff = buc.check_host_compatibility(rust_meta, jl_meta_diff)
        self.assertFalse(comp_diff["same_host"])
        self.assertIn("different_hosts", comp_diff["timing_authority_verdict"])

        # Same host
        comp_same = buc.check_host_compatibility(rust_meta, jl_meta_same)
        self.assertTrue(comp_same["same_host"])

if __name__ == "__main__":
    unittest.main()
