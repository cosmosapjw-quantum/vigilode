#!/usr/bin/env python3
"""Run only candidate-free F01--F05 formal checks into a fresh external directory."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
FORMAL_ROOT = REPO_ROOT / "research/audit2_stage_certificate_telemetry_20260831/formal"
DEFAULT_MATHLIB = Path("/home/cosmosapjw/Dropbox/bianchi/htt_base/formal_mathlib")

@dataclass(frozen=True)
class Backend:
    name: str
    source: Path
    version_argv: tuple[str, ...]
    expected_tokens: tuple[str, ...]

def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()

def file_record(path: Path) -> tuple[str, int]:
    return sha256(path), path.stat().st_size

def write_bytes(path: Path, contents: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)

def run_capture(argv: Sequence[str], cwd: Path | None) -> tuple[int, bytes, bytes]:
    try:
        completed = subprocess.run(list(argv), cwd=cwd, check=False, capture_output=True,
                                 env=os.environ.copy())
    except FileNotFoundError as exc:
        return 127, b"", (str(exc) + "\n").encode("utf-8")
    return completed.returncode, completed.stdout, completed.stderr

def version_text(backend: Backend, cwd: Path | None, logs: Path) -> tuple[bool, str]:
    exit_code, stdout, stderr = run_capture(backend.version_argv, cwd)
    write_bytes(logs / "versions" / f"{backend.name}.stdout", stdout)
    write_bytes(logs / "versions" / f"{backend.name}.stderr", stderr)
    combined = (stdout + stderr).decode("utf-8", errors="replace").strip()
    return exit_code == 0, combined.splitlines()[0] if combined else "UNAVAILABLE"

def execute(backend: Backend, argv: Sequence[str], cwd: Path | None, logs: Path,
            version: str) -> dict[str, object]:
    stdout_path = logs / "backends" / f"{backend.name}.stdout"
    stderr_path = logs / "backends" / f"{backend.name}.stderr"
    exit_code, stdout, stderr = run_capture(argv, cwd)
    write_bytes(stdout_path, stdout)
    write_bytes(stderr_path, stderr)
    decoded_stdout = stdout.decode("utf-8", errors="replace")
    missing = [token for token in backend.expected_tokens if token not in decoded_stdout]
    status = "PASS" if exit_code == 0 and not missing else "FORMAL_CHECK_FAILED"
    if exit_code == 127:
        status = "FORMAL_BACKEND_UNAVAILABLE"
    stdout_sha, stdout_bytes = file_record(stdout_path)
    stderr_sha, stderr_bytes = file_record(stderr_path)
    return {
        "backend": backend.name, "status": status, "version": version,
        "proof_source": str(backend.source.relative_to(REPO_ROOT)),
        "source_sha256": sha256(backend.source), "argv": list(argv), "exit_code": exit_code,
        "stdout_sha256": stdout_sha, "stdout_bytes": stdout_bytes,
        "stderr_sha256": stderr_sha, "stderr_bytes": stderr_bytes,
        "expected_tokens": list(backend.expected_tokens), "missing_tokens": missing,
    }

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True,
                        help="new external directory for raw logs and compiler products")
    parser.add_argument("--mathlib-project", type=Path, default=DEFAULT_MATHLIB)
    args = parser.parse_args()
    run_dir = args.run_dir.resolve()
    if run_dir.exists():
        print(f"refusing to overwrite existing external run directory: {run_dir}", file=sys.stderr)
        return 64
    if not args.mathlib_project.is_dir():
        print(f"FORMAL_BACKEND_UNAVAILABLE: mathlib project absent: {args.mathlib_project}", file=sys.stderr)
        return 2
    source_paths = (
        FORMAL_ROOT / "wolfram/check_stage_certificate.wl", FORMAL_ROOT / "sage/check_stage_majorant.sage",
        FORMAL_ROOT / "singular/check_jacobi_numerator.sing", FORMAL_ROOT / "lean/StageCertificate.lean",
        FORMAL_ROOT / "rocq/StageCertificate.v",
    )
    missing_sources = [path for path in source_paths if not path.is_file()]
    if missing_sources:
        print("FORMAL_CHECK_FAILED: missing source: " + ", ".join(map(str, missing_sources)), file=sys.stderr)
        return 3
    run_dir.mkdir(parents=True)
    logs, products = run_dir / "raw", run_dir / "compiler-products"
    lean_product, rocq_product = products / "lean", products / "rocq"
    sage_product = products / "sagemath"
    lean_product.mkdir(parents=True)
    rocq_product.mkdir(parents=True)
    sage_product.mkdir(parents=True)
    sage_source = sage_product / source_paths[1].name
    shutil.copyfile(source_paths[1], sage_source)
    if sha256(sage_source) != sha256(source_paths[1]):
        print("FORMAL_CHECK_FAILED: Sage execution copy hash mismatch", file=sys.stderr)
        return 3
    backends = (
        Backend("wolfram-language", source_paths[0], ("wolframscript", "-version"),
                ("F02_WOLFRAM_JACOBI_SQUARE_ZERO", "F03_WOLFRAM_APPROXIMATE_SOLVE_BOUND", "WOLFRAM_FORMAL_PASS")),
        Backend("sagemath", source_paths[1], ("sage", "--version"),
                ("F02_SAGE_JACOBI_SQUARE_ZERO", "F04_SAGE_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS", "SAGE_FORMAL_PASS")),
        Backend("singular", source_paths[2], ("Singular", "--version"),
                ("F02_SINGULAR_NUMERATOR_PATTERN_SQUARE_ZERO", "SINGULAR_FORMAL_PASS")),
        Backend("lean-mathlib", source_paths[3], ("lake", "env", "lean", "--version"),
                ("F01_LEAN_NILPOTENT_AND_FINITE_INVERSE", "F03_LEAN_APPROXIMATE_SOLVE_BOUND", "F04_LEAN_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS", "F05_LEAN_SAFE_INTERVAL_DECISIONS", "LEAN_FORMAL_PASS")),
        Backend("rocq", source_paths[4], ("coqc", "--version"),
                ("F01_ROCQ_NILPOTENT_AND_FINITE_INVERSE", "F03_ROCQ_APPROXIMATE_SOLVE_BOUND", "F04_ROCQ_FORWARD_MAJORANT_AND_WEIGHTED_BOUNDS", "F05_ROCQ_SAFE_INTERVAL_DECISIONS", "ROCQ_FORMAL_PASS")),
    )
    availability = {}
    for backend in backends:
        cwd = args.mathlib_project if backend.name == "lean-mathlib" else None
        availability[backend.name] = version_text(backend, cwd, logs)
    records = []
    for backend in backends:
        cwd = args.mathlib_project if backend.name == "lean-mathlib" else None
        if backend.name == "wolfram-language":
            argv = ("wolframscript", "-file", str(backend.source))
        elif backend.name == "sagemath":
            argv = ("sage", str(sage_source))
        elif backend.name == "singular":
            argv = ("Singular", str(backend.source))
        elif backend.name == "lean-mathlib":
            argv = ("lake", "env", "lean", "-R", str(REPO_ROOT), "-o", str(lean_product / "StageCertificate.olean"), str(backend.source))
        else:
            argv = ("coqc", "-q", "-noglob", "-o", str(rocq_product / "StageCertificate.vo"), str(backend.source))
        records.append(execute(backend, argv, cwd, logs, availability[backend.name][1]))
    if any(record["status"] == "FORMAL_BACKEND_UNAVAILABLE" for record in records):
        disposition, exit_code = "FORMAL_BACKEND_UNAVAILABLE", 2
    elif all(record["status"] == "PASS" for record in records):
        disposition, exit_code = "PASS", 0
    else:
        disposition, exit_code = "FORMAL_CHECK_FAILED", 3
    receipt = {"schema": "vigilode-audit2-stage-certificate-formal-receipt/v1", "status": disposition,
               "required_backends": [backend.name for backend in backends],
               "mathlib_project": str(args.mathlib_project.resolve()), "backends": records}
    receipt_path = run_dir / "formal_receipt.json"
    receipt_path.write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(f"FORMAL_RECEIPT={receipt_path}")
    print(f"FORMAL_STATUS={disposition}")
    return exit_code

if __name__ == "__main__":
    raise SystemExit(main())
