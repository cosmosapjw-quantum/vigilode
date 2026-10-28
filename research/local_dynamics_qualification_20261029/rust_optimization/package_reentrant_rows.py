#!/usr/bin/env python3
"""Pack all R1/R2 reentrant attempt output rows with original byte identities."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import re
import sys
from pathlib import Path

SCHEMA = "vigilode-reentrant-rows-supplement-v1"
GROUPS = ("reentrant-control", "reentrant-control-r2")
EXPECTED_ATTEMPTS_PER_GROUP = 68


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def json_bytes(obj: object) -> bytes:
    return (json.dumps(obj, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()


def read_stable(path: Path) -> bytes:
    before = path.stat()
    data = path.read_bytes()
    after = path.stat()
    if before.st_size != after.st_size or before.st_mtime_ns != after.st_mtime_ns:
        raise RuntimeError(f"source changed during packaging: {path}")
    return data


def compressed(raw: bytes) -> bytes:
    out = io.BytesIO()
    with gzip.GzipFile(fileobj=out, mode="wb", filename="", mtime=0, compresslevel=9) as stream:
        stream.write(raw)
    return out.getvalue()


def create_only(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as stream:
            stream.write(payload)
    except FileExistsError:
        if read_stable(path) != payload:
            raise RuntimeError(f"create-only conflict: {path}") from None


def source_identity(summary: dict[str, object], attempt: dict[str, object]) -> dict[str, object]:
    name = attempt["name"]
    match = re.search(r"-(baseline|candidate)(?:-r(?:4|12))?$", name)
    if match is None:
        raise RuntimeError(f"unrecognized attempt arm: {name}")
    arm = match.group(1)
    binary_paths = [arg for arg in attempt["argv"]
                    if isinstance(arg, str) and f"/target-{arm}" in arg
                    and "/release/examples/production_return_bench_20261027" in arg]
    if name.startswith("annotate-"):
        if binary_paths:
            raise RuntimeError(f"annotate unexpectedly invokes solver binary: {name}")
    elif len(binary_paths) != 1:
        raise RuntimeError(f"attempt executable/arm mismatch: {name}")
    return {
        "arm": arm,
        "source_sha": summary["source_sha"][arm],
        "binary_sha256": summary["binary_sha256"][arm],
        "binary_path": binary_paths[0] if binary_paths else None,
        "invokes_binary": bool(binary_paths),
    }


def collect(root: Path, output: Path) -> dict[str, object]:
    if not root.is_dir():
        raise RuntimeError(f"missing raw root: {root}")
    if output.resolve().is_relative_to(root.resolve()):
        raise RuntimeError("supplement output must be outside raw root")
    rows: list[dict[str, object]] = []
    summaries: list[dict[str, object]] = []
    for group in GROUPS:
        summary_path = root / group / "summary.json"
        summary_raw = read_stable(summary_path)
        summary = json.loads(summary_raw)
        attempts = summary.get("attempts")
        if not isinstance(attempts, list) or len(attempts) != EXPECTED_ATTEMPTS_PER_GROUP:
            raise RuntimeError(f"unexpected attempt count for {group}")
        summary_packet = Path("source") / group / "summary.json.gz"
        summary_gzip = compressed(summary_raw)
        create_only(output / summary_packet, summary_gzip)
        summaries.append({
            "group": group,
            "original_path": f"{group}/summary.json",
            "relative_path": f"{group}/summary.json",
            "original_bytes": len(summary_raw),
            "bytes": len(summary_raw),
            "original_sha256": sha(summary_raw),
            "sha256": sha(summary_raw),
            "packet_path": summary_packet.as_posix(),
            "packed_relative_path": summary_packet.as_posix(),
            "packet_bytes": len(summary_gzip),
            "packed_bytes": len(summary_gzip),
            "packet_sha256": sha(summary_gzip),
            "packed_sha256": sha(summary_gzip),
            "source_sha": summary["source_sha"],
            "binary_sha256": summary["binary_sha256"],
        })
        seen_names: set[str] = set()
        for attempt in attempts:
            name = attempt["name"]
            if name in seen_names:
                raise RuntimeError(f"duplicate attempt name: {group}/{name}")
            seen_names.add(name)
            identity = source_identity(summary, attempt)
            for kind in ("stdout", "stderr"):
                source = Path(attempt[f"{kind}_path"])
                if source != root / group / f"{name}.{kind}" or not source.is_file():
                    raise RuntimeError(f"missing or unrecognized {kind}: {group}/{name}")
                raw = read_stable(source)
                payload = compressed(raw)
                packet_path = Path("source") / group / f"{name}.{kind}.gz"
                create_only(output / packet_path, payload)
                rows.append({
                    "group": group,
                    "attempt_name": name,
                    "kind": kind,
                    "original_path": source.relative_to(root).as_posix(),
                    "relative_path": source.relative_to(root).as_posix(),
                    "original_bytes": len(raw),
                    "bytes": len(raw),
                    "original_sha256": sha(raw),
                    "sha256": sha(raw),
                    "packet_path": packet_path.as_posix(),
                    "packed_relative_path": packet_path.as_posix(),
                    "packet_bytes": len(payload),
                    "packed_bytes": len(payload),
                    "packet_sha256": sha(payload),
                    "packed_sha256": sha(payload),
                    "summary_sha256": sha(summary_raw),
                    "attempt_status": attempt["status"],
                    "attempt_exit_code": attempt["exit_code"],
                    **identity,
                })
    manifest = {
        "schema": SCHEMA,
        "source_root": str(root),
        "selection_rule": "Every stdout_path and stderr_path of all 68 attempts in each R1/R2 reentrant-control summary; summary files included. Gzip uses mtime=0 and empty filename.",
        "summaries": summaries,
        "rows": rows,
    }
    create_only(output / "SUPPLEMENT_MANIFEST.json", json_bytes(manifest))
    return verify(output)


def verify(output: Path) -> dict[str, object]:
    manifest = json.loads(read_stable(output / "SUPPLEMENT_MANIFEST.json"))
    if manifest.get("schema") != SCHEMA:
        raise RuntimeError("supplement schema mismatch")
    summaries = manifest["summaries"]
    rows = manifest["rows"]
    if len(summaries) != len(GROUPS) or len(rows) != 2 * EXPECTED_ATTEMPTS_PER_GROUP * len(GROUPS):
        raise RuntimeError("supplement incomplete")
    expected = {"SUPPLEMENT_MANIFEST.json"}
    for entry in [*summaries, *rows]:
        relative = entry["packet_path"]
        expected.add(relative)
        payload = read_stable(output / relative)
        original = gzip.decompress(payload)
        if len(payload) != entry["packet_bytes"] or sha(payload) != entry["packet_sha256"]:
            raise RuntimeError(f"packed byte mismatch: {relative}")
        if len(original) != entry["original_bytes"] or sha(original) != entry["original_sha256"]:
            raise RuntimeError(f"original byte mismatch: {relative}")
    actual = {p.relative_to(output).as_posix() for p in output.rglob("*") if p.is_file()}
    if actual != expected:
        raise RuntimeError(f"supplement file-set mismatch: missing={expected - actual}, extra={actual - expected}")
    for group in GROUPS:
        group_rows = [r for r in rows if r["group"] == group]
        if len(group_rows) != 2 * EXPECTED_ATTEMPTS_PER_GROUP:
            raise RuntimeError(f"missing rows for {group}")
        names = {r["attempt_name"] for r in group_rows}
        if len(names) != EXPECTED_ATTEMPTS_PER_GROUP:
            raise RuntimeError(f"duplicate or missing attempt for {group}")
        if any({r["kind"] for r in group_rows if r["attempt_name"] == name} != {"stdout", "stderr"} for name in names):
            raise RuntimeError(f"missing stdout/stderr pair for {group}")
    return {
        "schema": SCHEMA,
        "status": "PASS",
        "summaries": len(summaries),
        "attempts": len(rows) // 2,
        "output_rows": len(rows),
        "packet_files": len(expected),
        "packet_bytes": sum(p.stat().st_size for p in output.rglob("*") if p.is_file()),
        "manifest_sha256": sha(read_stable(output / "SUPPLEMENT_MANIFEST.json")),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path)
    parser.add_argument("--output", type=Path, default=Path(__file__).parent / "evidence_reentrant_rows")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    try:
        result = verify(args.output) if args.verify else collect(args.root.resolve(), args.output.resolve())
    except (OSError, ValueError, RuntimeError, KeyError, IndexError, gzip.BadGzipFile) as exc:
        print(json.dumps({"schema": SCHEMA, "status": "FAIL", "error": str(exc)}), file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
