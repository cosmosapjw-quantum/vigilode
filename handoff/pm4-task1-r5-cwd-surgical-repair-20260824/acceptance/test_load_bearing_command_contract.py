from __future__ import annotations

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
LOAD_BEARING = [
    ROOT / "CANONICAL_HANDOFF.md",
    ROOT / "acceptance" / "run_control_plane_preflight.sh",
]
RAW_OUTER_CHECK = re.compile(
    r"sha256sum\s+(?:--check|-c)\s+[^\n]*(?:PM4_R4_SIDECAR|\.tar\.gz\.sha256)",
    re.IGNORECASE,
)


class LoadBearingCommandContractTests(unittest.TestCase):
    def test_no_raw_outer_sidecar_check_remains(self) -> None:
        violations = []
        for path in LOAD_BEARING:
            self.assertTrue(path.is_file(), f"missing load-bearing file: {path}")
            if RAW_OUTER_CHECK.search(path.read_text(encoding="utf-8")):
                violations.append(str(path.relative_to(ROOT)))
        self.assertEqual(violations, [])

    def test_canonical_python_validator_is_load_bearing(self) -> None:
        combined = "\n".join(path.read_text(encoding="utf-8") for path in LOAD_BEARING)
        self.assertIn("test_archive_authority_contract.py", combined)
        self.assertIn("run_control_plane_preflight.sh", combined)


if __name__ == "__main__":
    unittest.main()
