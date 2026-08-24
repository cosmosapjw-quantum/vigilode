from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "acceptance" / "test_archive_authority_contract.py"
PREFLIGHT = ROOT / "acceptance" / "run_control_plane_preflight.sh"
CANONICAL = "6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333"
WITHDRAWN = "b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095"
ARCHIVE_NAME = "VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz"


def run_validator(archive: Path, sidecar: Path, cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "python3",
            str(VALIDATOR),
            "--archive",
            str(archive),
            "--sidecar",
            str(sidecar),
        ],
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


class ArchiveGateCwdContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        archive = os.environ.get("PM4_R4_ARCHIVE")
        sidecar = os.environ.get("PM4_R4_SIDECAR")
        if not archive or not sidecar:
            raise AssertionError("PM4_R4_ARCHIVE and PM4_R4_SIDECAR are required")
        cls.archive = Path(archive).resolve()
        cls.sidecar = Path(sidecar).resolve()
        if not cls.archive.is_file() or not cls.sidecar.is_file():
            raise AssertionError("canonical R4 archive or sidecar missing")

    def test_canonical_validator_passes_from_three_unrelated_cwds(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cwd_cases = [ROOT, ROOT.parent, Path(tmp)]
            for cwd in cwd_cases:
                with self.subTest(cwd=cwd):
                    result = run_validator(self.archive, self.sidecar, cwd)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_control_plane_preflight_exists_and_passes_from_three_cwds(self) -> None:
        self.assertTrue(PREFLIGHT.is_file(), f"missing canonical preflight: {PREFLIGHT}")
        with tempfile.TemporaryDirectory() as tmp:
            cwd_cases = [ROOT, ROOT.parent, Path(tmp)]
            env = os.environ.copy()
            env["PM4_R4_ARCHIVE"] = str(self.archive)
            env["PM4_R4_SIDECAR"] = str(self.sidecar)
            for cwd in cwd_cases:
                with self.subTest(cwd=cwd):
                    result = subprocess.run(
                        ["bash", str(PREFLIGHT)],
                        cwd=cwd,
                        env=env,
                        text=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_wrong_sidecar_hash_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            sidecar = Path(tmp) / "bad.sha256"
            sidecar.write_text(f"{'0' * 64}  {ARCHIVE_NAME}\n", encoding="utf-8")
            result = run_validator(self.archive, sidecar, Path(tmp))
            self.assertNotEqual(result.returncode, 0)

    def test_withdrawn_hash_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            sidecar = Path(tmp) / "withdrawn.sha256"
            sidecar.write_text(f"{WITHDRAWN}  {ARCHIVE_NAME}\n", encoding="utf-8")
            result = run_validator(self.archive, sidecar, Path(tmp))
            self.assertNotEqual(result.returncode, 0)

    def test_sidecar_wrong_basename_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            sidecar = Path(tmp) / "wrong-name.sha256"
            sidecar.write_text(f"{CANONICAL}  unrelated.tar.gz\n", encoding="utf-8")
            result = run_validator(self.archive, sidecar, Path(tmp))
            self.assertNotEqual(result.returncode, 0)

    def test_sidecar_multiple_active_records_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            sidecar = Path(tmp) / "multiple.sha256"
            sidecar.write_text(
                f"{CANONICAL}  {ARCHIVE_NAME}\n{CANONICAL}  duplicate.tar.gz\n",
                encoding="utf-8",
            )
            result = run_validator(self.archive, sidecar, Path(tmp))
            self.assertNotEqual(result.returncode, 0)

    def test_missing_archive_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            missing = Path(tmp) / ARCHIVE_NAME
            result = run_validator(missing, self.sidecar, Path(tmp))
            self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
