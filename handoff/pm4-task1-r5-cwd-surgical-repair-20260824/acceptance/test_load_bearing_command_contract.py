from __future__ import annotations

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
LOAD_BEARING = [
    ROOT / "AGENTS.md",
    ROOT / "README_FIRST.md",
    ROOT / "AUDIT_COMPILED_EXEC_PLAN.yaml",
    ROOT / "IMPLEMENTER_PROMPT.md",
    ROOT / "FRESH_REVIEW_PROMPT.md",
    ROOT / "acceptance" / "README.md",
    ROOT / "acceptance" / "run_acceptance.sh",
]
PREFLIGHT = ROOT / "acceptance" / "run_control_plane_preflight.sh"
RAW_OUTER_CHECK = re.compile(
    r"sha256sum\s+-c[^\n]*(?:R4_SIDECAR|TASK1_SCHEMA_BOUNDARY_KIT_R4[^\n]*\.sha256|\.tar\.gz\.sha256)",
    re.IGNORECASE,
)


class LoadBearingCommandContractTests(unittest.TestCase):
    def test_no_load_bearing_raw_outer_sidecar_check(self) -> None:
        offenders: list[str] = []
        for path in LOAD_BEARING:
            self.assertTrue(path.is_file(), f"missing load-bearing file: {path}")
            text = path.read_text(encoding="utf-8")
            if RAW_OUTER_CHECK.search(text):
                offenders.append(str(path.relative_to(ROOT)))
        self.assertEqual(
            offenders,
            [],
            "raw outer sidecar checks have implicit CWD semantics: " + ", ".join(offenders),
        )

    def test_canonical_preflight_is_repo_location_independent(self) -> None:
        self.assertTrue(PREFLIGHT.is_file(), f"missing canonical preflight: {PREFLIGHT}")
        text = PREFLIGHT.read_text(encoding="utf-8")
        self.assertIn('SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"', text)
        self.assertIn('HANDOFF_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"', text)
        self.assertNotRegex(text, RAW_OUTER_CHECK)
        self.assertIn("test_archive_authority_contract.py", text)


if __name__ == "__main__":
    unittest.main()
