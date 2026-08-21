#!/usr/bin/env python3
"""Build a deterministic SHA-256 manifest for the complete v3.6 artifact tree."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--implementation-commit", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output_resolved = args.output.resolve()
    files = []
    for path in sorted(args.root.rglob("*")):
        if not path.is_file() or path.resolve() == output_resolved:
            continue
        if "__pycache__" in path.parts or path.suffix == ".pyc":
            continue
        files.append(
            {
                "path": str(path.relative_to(args.root)),
                "bytes": path.stat().st_size,
                "sha256": digest(path),
            }
        )
    manifest = {
        "schema": "vigilode-v36-artifact-manifest-v1",
        "implementation_checkpoint": args.implementation_commit,
        "file_count": len(files),
        "files": files,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("V36_ARTIFACT_MANIFEST_COMPLETE")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
