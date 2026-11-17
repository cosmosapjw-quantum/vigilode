#!/usr/bin/env python3
"""Loop L1 node V02 (trial merge): protected-blob delta.

For every path in CHECKPOINT.json "protected_blob_sha1" this records the git blob id at
the loop base, at the merge commit and at the evaluated head, whether it changed, and the
numstat of base..head.  Everything is read from git; nothing is typed in.

Usage: python3 -I make_blob_delta.py <base> <merge_commit> <head> <out.json>
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def git(*args: str) -> str:
    out = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {out.stderr.strip()}")
    return out.stdout


def blob(rev: str, path: str) -> str | None:
    out = subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", f"{rev}:{path}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return out.stdout.strip() if out.returncode == 0 else None


def numstat(a: str, b: str, path: str) -> dict:
    line = git("diff", "--numstat", a, b, "--", path).strip()
    if not line:
        return {"added": 0, "deleted": 0}
    added, deleted, _ = line.split("\t", 2)
    return {"added": int(added), "deleted": int(deleted)}


def main() -> int:
    base, merge, head, out_path = sys.argv[1:5]
    base, merge, head = (git("rev-parse", r).strip() for r in (base, merge, head))
    checkpoint = json.loads(
        git("show", f"{base}:research/production_loop_20261116/CHECKPOINT.json")
    )
    rows = []
    for path, recorded in checkpoint["protected_blob_sha1"].items():
        b, m, h = blob(base, path), blob(merge, path), blob(head, path)
        rows.append(
            {
                "path": path,
                "checkpoint_blob": recorded,
                "base_blob": b,
                "base_matches_checkpoint": b == recorded,
                "merge_commit_blob": m,
                "head_blob": h,
                "changed_by_merge_commit": b != m,
                "changed_at_head": b != h,
                "restored_by_fixups": b != m and b == h,
                "numstat_base_to_merge_commit": numstat(base, merge, path),
                "numstat_base_to_head": numstat(base, head, path),
            }
        )
    changed = [r["path"] for r in rows if r["changed_at_head"]]
    out = {
        "schema": "vigilode.loop-l1.trial-merge.blob-delta.v1",
        "generator": "research/production_loop_20261116/trial_merge/make_blob_delta.py",
        "base_commit": base,
        "merge_commit": merge,
        "head_commit": head,
        "merged_commit": git("rev-parse", f"{merge}^2").strip(),
        "protected_paths": len(rows),
        "base_mismatches_checkpoint": [
            r["path"] for r in rows if not r["base_matches_checkpoint"]
        ],
        "changed_by_merge_commit": [r["path"] for r in rows if r["changed_by_merge_commit"]],
        "restored_by_fixups": [r["path"] for r in rows if r["restored_by_fixups"]],
        "changed_at_head": changed,
        "unchanged_at_head": [r["path"] for r in rows if not r["changed_at_head"]],
        "rows": rows,
    }
    Path(out_path).write_text(json.dumps(out, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    print(f"{len(rows)} protected paths, {len(changed)} changed at head -> {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
