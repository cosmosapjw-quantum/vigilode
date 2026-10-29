#!/usr/bin/env python3
"""Create and verify a compact, byte-linked local Rust optimization packet.

The source directory is read only. Existing packet files are never replaced:
repeating the command accepts identical bytes and rejects a changed input.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import sys
from pathlib import Path

SCHEMA = "vigilode-rust-optimization-evidence-v1"
MAX_SOURCE_FILE = 2 * 1024 * 1024
MAX_PACKET_BYTES = 5 * 1024 * 1024
COMPRESS_AT = 16 * 1024
EXCLUDED_DIRS = {"baseline", "baseline-harness", "packet-v1"}
TOP_LEVEL_DIRS_WITH_RECEIPTS = {
    "baseline-preflight",
    "build-baseline",
    "build-baseline-final",
    "build-baseline-r1",
    "build-candidate",
    "build-candidate-r2",
    "existing-timefd-regression",
    "fd-facade-contracts",
    "fd-green",
    "fd-red",
    "fd-red2",
    "fd-red3",
    "fd-unit-green",
    "guard-fatal-regression",
    "light-wrapper",
    "light-wrapper-r2",
    "r2-clone-workspace",
    "r2-fd-final-integration",
    "r2-fd-final-unit",
    "r2-strict-focused-clippy",
    "r2-timefd-contracts",
    "r2-timefd-unit",
    "reentrant-wrapper",
    "reentrant-wrapper-r2",
    "strict-focused-clippy",
    "strict-focused-clippy2",
    "strict-focused-clippy3",
    "timefd-green",
    "timefd-green-final",
    "timefd-red",
    "timefd-red2",
    "timefd-red3",
    "timefd-unit-green",
    "timefd-unit-green2",
}


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def encoded_json(obj: object) -> bytes:
    return (json.dumps(obj, sort_keys=True, indent=2, ensure_ascii=False) + "\n").encode()


def read_stable(path: Path) -> bytes:
    before = path.stat()
    data = path.read_bytes()
    after = path.stat()
    if before.st_size != after.st_size or before.st_mtime_ns != after.st_mtime_ns:
        raise RuntimeError(f"source changed during packaging: {path}")
    return data


def original_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for folder, dirs, names in os.walk(root, followlinks=False):
        dirs[:] = sorted(
            d
            for d in dirs
            if d not in EXCLUDED_DIRS and not d.startswith("target-") and d != ".git"
        )
        for name in sorted(names):
            if name == "package-receipt-final.json":
                continue  # This command's own receipt cannot index itself.
            path = Path(folder) / name
            if path.is_symlink():
                raise RuntimeError(f"unexpected symlink: {path}")
            if not path.is_file():
                continue
            if path.stat().st_size <= MAX_SOURCE_FILE:
                files.append(path)
    return sorted(files, key=lambda p: p.relative_to(root).as_posix())


def include(rel: Path) -> bool:
    parts = rel.parts
    if len(parts) == 1:
        return rel.suffix in {".json", ".txt", ".patch", ".rs", ".py"}
    group = parts[0]
    name = rel.name
    if group in TOP_LEVEL_DIRS_WITH_RECEIPTS:
        return name in {"RECEIPT.json", "SOURCE_INPUTS.json", "stdout.log", "stderr.log"}
    if group in {"light-final", "light-final-r2"}:
        return rel.suffix in {".json", ".txt", ".resource"}
    if group in {"annotations-corrected", "annotations-corrected-r2"}:
        return rel.suffix == ".txt" or name == "COMMANDS.json"
    if group in {"heaptrack", "heaptrack-r2"}:
        return name.endswith("-allocators.txt") or name in {"COMMANDS.json", "PRINT_COMMANDS.json"}
    if group in {"rss-instrumented", "rss-instrumented-r2"}:
        return rel.suffix == ".resource" or name == "COMMANDS.json"
    if group in {"reentrant-control", "reentrant-control-r2"}:
        return rel.suffix == ".json" or name.startswith("annotate-") and rel.suffix == ".stdout"
    if group.startswith("fake-runner-regression"):
        return name in {"manifest.json", "e2e-summary.json"}
    if group.startswith("reviewcheck-") or group.startswith("runlight-check-"):
        return rel.suffix == ".json"
    return False


def gzip_bytes(raw: bytes) -> bytes:
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0, compresslevel=9) as stream:
        stream.write(raw)
    return output.getvalue()


def create_only(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as stream:
            stream.write(payload)
    except FileExistsError:
        existing = read_stable(path)
        if existing != payload:
            raise RuntimeError(f"create-only packet conflict: {path}") from None


def binary_index(root: Path, rows: list[dict[str, object]]) -> list[dict[str, object]]:
    seen: set[tuple[str, str]] = set()
    binaries: list[dict[str, object]] = []

    def visit(value: object, source: str) -> None:
        if isinstance(value, dict):
            path, digest = value.get("path"), value.get("sha256")
            if isinstance(path, str) and isinstance(digest, str) and path.startswith(str(root / "target-")):
                key = (path, digest)
                if key not in seen:
                    seen.add(key)
                    binaries.append({"path": path, "sha256": digest, "source_identity": source})
            for child in value.values():
                visit(child, source)
        elif isinstance(value, list):
            for child in value:
                visit(child, source)

    for row in rows:
        rel = str(row["relative_path"])
        if "/" not in rel and (rel.startswith("IDENTITY") or rel.startswith("MEASUREMENT_IDENTITY")):
            visit(json.loads((root / rel).read_text()), rel)
    return sorted(binaries, key=lambda b: (str(b["path"]), str(b["sha256"])))


def collect(root: Path, packet: Path) -> dict[str, object]:
    if not root.is_dir():
        raise RuntimeError(f"missing source root: {root}")
    if packet.resolve().is_relative_to(root.resolve()):
        raise RuntimeError("packet must be outside the source root")
    entries: list[dict[str, object]] = []
    for path in original_files(root):
        rel = path.relative_to(root)
        raw = read_stable(path)
        row: dict[str, object] = {
            "relative_path": rel.as_posix(),
            "bytes": len(raw),
            "sha256": sha(raw),
            "packaged": include(rel),
        }
        if row["packaged"]:
            compressed = len(raw) >= COMPRESS_AT or rel.as_posix() in {
                "light-final/manifest.json",
                "light-final-r2/manifest.json",
                "reentrant-control/summary.json",
                "reentrant-control-r2/summary.json",
            }
            target = Path("source") / (rel.as_posix() + (".gz" if compressed else ""))
            create_only(packet / target, gzip_bytes(raw) if compressed else raw)
            row["packet_path"] = target.as_posix()
            row["encoding"] = "gzip-mtime-0" if compressed else "identity"
        entries.append(row)
    index = {
        "schema": SCHEMA,
        "local_source_root": str(root),
        "selection_rule": {
            "indexed": "All regular files at most 2 MiB under root, excluding baseline/baseline-harness worktrees, preserved packet-v1, target-* caches, .git, and this command's final receipt.",
            "packaged": "Top-level identity/source patches; all build and test receipts/logs/inputs; R1/R2 light manifests and compact JSON; corrected R1/R2 function tables; R1/R2 control summaries/annotated tables; allocator text; RSS resource logs; command metadata; representative runner-failure manifests.",
            "excluded_payload": "ELF binaries, Cargo target caches, Callgrind *.out or *.callgrind raw, heaptrack *.zst, full timing stdout and fake runner bulk files. Their existing small-file hash/size/path remains in this index where applicable.",
            "gzip": "Deterministic gzip, mtime=0, empty filename, level=9; original and gzip identities recorded separately.",
        },
        "files": entries,
    }
    create_only(packet / "RAW_INDEX.json", encoded_json(index))
    create_only(packet / "BINARY_INDEX.json", encoded_json({"schema": SCHEMA, "binaries": binary_index(root, entries)}))
    files = sorted(p for p in packet.rglob("*") if p.is_file() and p.name != "PACKET_MANIFEST.json")
    manifest = {
        "schema": SCHEMA,
        "packet_files": [
            {"relative_path": p.relative_to(packet).as_posix(), "bytes": p.stat().st_size, "sha256": sha(read_stable(p))}
            for p in files
        ],
    }
    create_only(packet / "PACKET_MANIFEST.json", encoded_json(manifest))
    return verify(packet)


def verify(packet: Path) -> dict[str, object]:
    index = json.loads(read_stable(packet / "RAW_INDEX.json"))
    manifest = json.loads(read_stable(packet / "PACKET_MANIFEST.json"))
    if index.get("schema") != SCHEMA or manifest.get("schema") != SCHEMA:
        raise RuntimeError("packet schema mismatch")
    expected = {row["relative_path"]: row for row in manifest["packet_files"]}
    actual = {p.relative_to(packet).as_posix() for p in packet.rglob("*") if p.is_file() and p.name != "PACKET_MANIFEST.json"}
    if actual != set(expected):
        raise RuntimeError(f"packet file-set mismatch: missing={set(expected)-actual} extra={actual-set(expected)}")
    for relative, row in expected.items():
        payload = read_stable(packet / relative)
        if len(payload) != row["bytes"] or sha(payload) != row["sha256"]:
            raise RuntimeError(f"packet byte mismatch: {relative}")
    packaged = 0
    for row in index["files"]:
        if not row["packaged"]:
            continue
        packaged += 1
        payload = read_stable(packet / row["packet_path"])
        original = gzip.decompress(payload) if row["encoding"] == "gzip-mtime-0" else payload
        if len(original) != row["bytes"] or sha(original) != row["sha256"]:
            raise RuntimeError(f"source byte identity mismatch: {row['relative_path']}")
    total = sum(p.stat().st_size for p in packet.rglob("*") if p.is_file())
    if total > MAX_PACKET_BYTES:
        raise RuntimeError(f"packet exceeds 5 MiB: {total} bytes")
    return {
        "schema": SCHEMA,
        "status": "PASS",
        "indexed_files": len(index["files"]),
        "packaged_source_files": packaged,
        "packet_files": len(expected) + 1,
        "packet_bytes": total,
        "packet_manifest_sha256": sha(read_stable(packet / "PACKET_MANIFEST.json")),
        "raw_index_sha256": sha(read_stable(packet / "RAW_INDEX.json")),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, help="Immutable local raw evidence root")
    parser.add_argument("--output", type=Path, default=Path(__file__).parent / "evidence")
    parser.add_argument("--verify", action="store_true", help="Verify an existing packet without touching it")
    args = parser.parse_args()
    try:
        result = verify(args.output) if args.verify else collect(args.root.resolve(), args.output.resolve())
    except (OSError, ValueError, RuntimeError, KeyError, gzip.BadGzipFile) as exc:
        print(json.dumps({"schema": SCHEMA, "status": "FAIL", "error": str(exc)}), file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
