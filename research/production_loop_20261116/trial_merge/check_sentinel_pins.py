#!/usr/bin/env python3
"""Loop L1 node V02 (trial merge): evaluate the pinned source hashes of the two Peer
sentinel tests (`g3_protected_sources_are_byte_identical` in ppsw1_6b_contracts.rs and
`h2_protected_sources_are_byte_identical` in ptsw2b_contracts.rs) at several commits.

The tests stop at the first mismatch; this lists every pinned file, so the full set of
mismatching files is known.  Pins are read from the test sources at each commit, file
bytes from `git show`.  Nothing is typed in.

Usage: python3 -I check_sentinel_pins.py <out.json> <label=rev> [<label=rev> ...]
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CRATE = "crates/rodas5p-integrators"
TESTS = {
    "g3_protected_sources_are_byte_identical": f"{CRATE}/tests/ppsw1_6b_contracts.rs",
    "h2_protected_sources_are_byte_identical": f"{CRATE}/tests/ptsw2b_contracts.rs",
}
PAIR = re.compile(r'\(\s*"([^"]+)",\s*"([0-9a-f]{64})",?\s*\)')
LIB_PIN = re.compile(r'sha256_hex\(lib\.replace\(marker, ""\)\.as_bytes\(\)\),\s*"([0-9a-f]{64})"')


def show(rev: str, path: str) -> bytes | None:
    out = subprocess.run(["git", "show", f"{rev}:{path}"], cwd=ROOT, capture_output=True)
    return out.stdout if out.returncode == 0 else None


def norm(path: str) -> str:
    parts: list[str] = []
    for p in f"{CRATE}/{path}".split("/"):
        if p == "..":
            parts.pop()
        else:
            parts.append(p)
    return "/".join(parts)


def body(src: str, fn: str) -> str:
    start = src.index(f"fn {fn}()")
    end = src.find("\n}\n", start)
    return src[start:end]


def evaluate(rev: str) -> dict:
    full = subprocess.run(
        ["git", "rev-parse", rev], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout.strip()
    tests = {}
    for fn, test_path in TESTS.items():
        src = show(full, test_path).decode("utf-8")
        text = body(src, fn)
        rows = []
        for rel, want in PAIR.findall(text):
            data = show(full, norm(rel))
            got = hashlib.sha256(data).hexdigest() if data is not None else None
            rows.append({"path": norm(rel), "pinned": want, "actual": got, "match": got == want})
        entry = {
            "test_file": test_path,
            "pinned_files": len(rows),
            "mismatching": [r["path"] for r in rows if not r["match"]],
            "rows": rows,
        }
        m = LIB_PIN.search(text)
        if m:
            lib = show(full, f"{CRATE}/src/lib.rs").decode("utf-8")
            got = hashlib.sha256(lib.replace("pub mod ppsw1;\n", "").encode("utf-8")).hexdigest()
            entry["lib_rs_minus_ppsw1_line"] = {
                "pinned": m.group(1),
                "actual": got,
                "match": got == m.group(1),
            }
        tests[fn] = entry
    return {"rev": full, "tests": tests}


def main() -> int:
    out_path = Path(sys.argv[1])
    result = {
        "schema": "vigilode.loop-l1.trial-merge.sentinel-pins.v1",
        "generator": "research/production_loop_20261116/trial_merge/check_sentinel_pins.py",
        "commits": {},
    }
    for arg in sys.argv[2:]:
        label, _, rev = arg.partition("=")
        result["commits"][label] = evaluate(rev)
        for fn, t in result["commits"][label]["tests"].items():
            lib = t.get("lib_rs_minus_ppsw1_line")
            print(
                f"{label} {fn}: {len(t['mismatching'])}/{t['pinned_files']} mismatching "
                f"{t['mismatching']}" + (f" lib_rs_rule_match={lib['match']}" if lib else "")
            )
    out_path.write_text(json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
