from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import subprocess
import tarfile
import tempfile

CANONICAL = "6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333"
PATCH = "705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3"
SCRIPT = "63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7"
WITHDRAWN = "b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", required=True)
    parser.add_argument("--sidecar", required=True)
    args = parser.parse_args()

    archive = Path(args.archive).resolve()
    sidecar = Path(args.sidecar).resolve()
    actual = sha256(archive)
    if actual != CANONICAL:
        raise AssertionError(f"archive SHA mismatch: {actual} != {CANONICAL}")
    if actual == WITHDRAWN:
        raise AssertionError("withdrawn hash must not be accepted")

    expected_from_sidecar = sidecar.read_text(encoding="utf-8").split()[0]
    if expected_from_sidecar != CANONICAL:
        raise AssertionError("sidecar does not bind canonical hash")

    with tempfile.TemporaryDirectory() as tmp:
        with tarfile.open(archive, "r:gz") as archive_file:
            archive_file.extractall(tmp, filter="data")
        roots = [path for path in Path(tmp).iterdir() if path.is_dir()]
        if len(roots) != 1:
            raise AssertionError("archive must have one root directory")
        root = roots[0]
        subprocess.run(["sha256sum", "-c", "SHA256SUMS"], cwd=root, check=True)
        if sha256(root / "PM4_TASK1_SCHEMA_BOUNDARY.patch") != PATCH:
            raise AssertionError("sealed Task-1 patch mismatch")
        if sha256(root / "publish_pm4_task1.sh") != SCRIPT:
            raise AssertionError("R4 publication script mismatch")

    print("PASS: canonical R4 outer hash, sidecar, internal manifest, patch, and script")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
