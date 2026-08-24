from __future__ import annotations

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
PATCH = ROOT / "payload" / "PM4_TASK1_SCHEMA_BOUNDARY.patch"
EXPECTED = {
    "crates/rodas5p-integrators/src/lib.rs",
    "crates/rodas5p-integrators/src/v38d_performance_tournament.rs",
    "crates/rodas5p-integrators/tests/v38d_performance_probe_contracts.rs",
}
FORBIDDEN_PREFIXES = ("Cargo.toml", "Cargo.lock", ".cargo/", "crates/rodas5p-core/", "crates/rodas5p-krylov/")


class PayloadContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = PATCH.read_text(encoding="utf-8")

    def test_patch_touches_exact_three_rust_paths(self) -> None:
        paths = set(re.findall(r"^diff --git a/(.+?) b/(.+)$", self.text, flags=re.MULTILINE))
        flattened = {left for left, right in paths if left == right}
        self.assertEqual(flattened, EXPECTED)
        self.assertEqual(len(paths), len(EXPECTED))

    def test_patch_avoids_forbidden_paths(self) -> None:
        for path in EXPECTED:
            self.assertFalse(path.startswith(FORBIDDEN_PREFIXES))
        for prefix in FORBIDDEN_PREFIXES:
            self.assertNotIn(f"diff --git a/{prefix}", self.text)

    def test_schema_boundary_is_explicitly_non_authority(self) -> None:
        required = [
            '"vigilode-v38d-exploratory-probe-v1"',
            '"EXPLORATORY_NOT_TIMING_AUTHORITY"',
            "timing_authority: false",
            "speedup_claim_authorized: false",
            "active_switching_authorized: false",
            "policy_retuning_authorized: false",
            "release_claim_authorized: false",
            "n2048_authorized: false",
            '"v3.8-D probe case not implemented"',
        ]
        for token in required:
            self.assertIn(token, self.text)

    def test_exact_case_and_repetition_contract_is_present(self) -> None:
        for case in [
            "stiff-diagonal-96",
            "nonnormal-jordan-96",
            "oscillatory-blocks-96",
            "diffusion-like-192",
            "mixed-forcing-192",
        ]:
            self.assertIn(case, self.text)
        self.assertIn("V38D_WARMUP_REPETITIONS: usize = 1", self.text)
        self.assertIn("V38D_MEASURED_REPETITIONS: usize = 7", self.text)


if __name__ == "__main__":
    unittest.main()
