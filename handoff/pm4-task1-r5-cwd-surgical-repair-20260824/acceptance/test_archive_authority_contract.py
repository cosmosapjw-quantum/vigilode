from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile

CANONICAL = "6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333"
WITHDRAWN = "b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095"
ARCHIVE_NAME = "VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz"
PATCH = "705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3"
SCRIPT = "63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7"
RECORD = re.compile(r"^([0-9A-Fa-f]{64})[ \t]+(.+?)\s*$")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_sidecar(path: Path) -> str:
    active = [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    if len(active) != 1:
        raise AssertionError(
            f"sidecar must contain exactly one active record; found {len(active)}"
        )
    match = RECORD.fullmatch(active[0])
    if not match:
        raise AssertionError("sidecar record is not in SHA-256 check format")
    declared, filename = match.groups()
    filename = filename.strip()
    if filename.startswith("*"):
        filename = filename[1:]
    if "/" in filename or "\\" in filename or filename != ARCHIVE_NAME:
        raise AssertionError(f"sidecar filename mismatch: {filename!r}")
    return declared.lower()


def validate(archive: Path, sidecar: Path) -> None:
    archive = archive.resolve()
    sidecar = sidecar.resolve()
    if not archive.is_file():
        raise AssertionError(f"archive missing: {archive}")
    if not sidecar.is_file():
        raise AssertionError(f"sidecar missing: {sidecar}")
    if archive.name != ARCHIVE_NAME:
        raise AssertionError(f"archive basename mismatch: {archive.name}")

    declared = parse_sidecar(sidecar)
    if declared == WITHDRAWN:
        raise AssertionError("withdrawn archive hash must not be accepted")
    if declared != CANONICAL:
        raise AssertionError(f"sidecar hash mismatch: {declared} != {CANONICAL}")
    actual = sha256(archive)
    if actual != CANONICAL:
        raise AssertionError(f"archive SHA mismatch: {actual} != {CANONICAL}")

    with tempfile.TemporaryDirectory() as tmp:
        with tarfile.open(archive, "r:gz") as archive_file:
            try:
                archive_file.extractall(tmp, filter="data")
            except TypeError:
                archive_file.extractall(tmp)
        roots = [path for path in Path(tmp).iterdir() if path.is_dir()]
        if len(roots) != 1:
            raise AssertionError("archive must contain exactly one root directory")
        root = roots[0]
        subprocess.run(["sha256sum", "-c", "SHA256SUMS"], cwd=root, check=True)
        if sha256(root / "PM4_TASK1_SCHEMA_BOUNDARY.patch") != PATCH:
            raise AssertionError("sealed Task-1 patch mismatch")
        if sha256(root / "publish_pm4_task1.sh") != SCRIPT:
            raise AssertionError("sealed R4 publication script mismatch")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", required=True)
    parser.add_argument("--sidecar", required=True)
    args = parser.parse_args()
    validate(Path(args.archive), Path(args.sidecar))
    print("PASS: CWD-independent R4 archive authority chain")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
