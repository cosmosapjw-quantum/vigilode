from __future__ import annotations

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
LAUNCHER = ROOT / "CODEX_LAUNCHER.md"
RECONSTRUCT = ROOT / "RECONSTRUCT_AND_VERIFY.sh"
README = ROOT / "README.md"


class LauncherDeliveryContractTests(unittest.TestCase):
    def test_repository_local_launcher_and_reconstructor_exist(self) -> None:
        self.assertTrue(LAUNCHER.is_file(), f"missing repository-local launcher: {LAUNCHER}")
        self.assertTrue(RECONSTRUCT.is_file(), f"missing reconstruction script: {RECONSTRUCT}")
        self.assertTrue(README.is_file(), f"missing handoff README: {README}")

    def test_launcher_has_no_chat_sandbox_dependency(self) -> None:
        text = LAUNCHER.read_text(encoding="utf-8")
        self.assertNotIn("sandbox:/", text)
        self.assertNotIn("/mnt/data", text)
        self.assertNotIn("VIGILODE_PM4_CODEX_RERUN_PROMPT_CWD_SURGICAL_20260824.md", text)
        self.assertIn("RECONSTRUCT_AND_VERIFY.sh", text)
        self.assertIn("CANONICAL_HANDOFF.md", text)
        self.assertIn("IMPLEMENTER_PROMPT.md", text)

    def test_reconstructor_is_location_independent_and_fail_closed(self) -> None:
        text = RECONSTRUCT.read_text(encoding="utf-8")
        self.assertIn('SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"', text)
        self.assertIn("set -euo pipefail", text)
        self.assertIn("sha256sum -c", text)
        self.assertIn("base64 -d", text)
        self.assertIn("tar -tzf", text)
        self.assertRegex(text, re.compile(r"part-\{00\.\.06\}|for part in 00 01 02 03 04 05 06"))

    def test_readme_points_to_repository_local_launcher(self) -> None:
        text = README.read_text(encoding="utf-8")
        self.assertIn('cat "$HANDOFF_ROOT/CODEX_LAUNCHER.md"', text)
        self.assertIn('bash "$HANDOFF_ROOT/RECONSTRUCT_AND_VERIFY.sh"', text)


if __name__ == "__main__":
    unittest.main()
