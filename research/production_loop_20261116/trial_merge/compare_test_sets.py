#!/usr/bin/env python3
"""Loop L1 node V02 (trial merge): compare per-test outcomes of two raw `cargo test` logs.

Each test is keyed by (test binary, test name); the outcome is the word libtest prints
after "...".  Reports tests of the baseline that are missing after the merge, tests whose
outcome changed, and how many tests are new.  Read from the logs only.

Usage: python3 -I compare_test_sets.py <out.json> <label> <baseline.log> <merged.log> [...]
       (triples of label, baseline log, merged log)
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

RUNNING = re.compile(r"^\s+Running (?:unittests )?(\S+) \((\S+)\)|^\s+Doc-tests (\S+)")


def binary_label(m: re.Match) -> str:
    """`<executable name without hash>:<source path>`, or `doc-tests <crate>`."""
    if m.group(3):
        return f"doc-tests {m.group(3)}"
    exe = m.group(2).rsplit("/", 1)[-1].rsplit("-", 1)[0]
    return f"{exe}:{m.group(1)}"
TEST = re.compile(r"^test (.+?) \.\.\. (ok|FAILED|ignored)\b")


def outcomes(path: Path) -> dict:
    current, out = None, {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = RUNNING.match(line)
        if m:
            current = binary_label(m)
            continue
        m = TEST.match(line)
        if m and current is not None:
            name = m.group(1)
            # Doc-test names carry a line number that moves when a file is edited.
            if current.startswith("doc-tests"):
                name = re.sub(r" \(line \d+\)", "", name)
            key = f"{current} :: {name}"
            out[key] = out.get(key, []) + [m.group(2)]
    return out


def main() -> int:
    out_path = Path(sys.argv[1])
    root = Path(__file__).resolve().parents[3]
    args = sys.argv[2:]
    rows = []
    for i in range(0, len(args), 3):
        label, base_log, merged_log = args[i], Path(args[i + 1]), Path(args[i + 2])
        base, merged = outcomes(base_log), outcomes(merged_log)
        missing = sorted(k for k in base if k not in merged)
        changed = [
            {"test": k, "baseline": base[k], "merged": merged[k]}
            for k in sorted(base)
            if k in merged and sorted(base[k]) != sorted(merged[k])
        ]
        new = sorted(k for k in merged if k not in base)
        new_binaries = sorted({k.split(" :: ")[0] for k in new} - {k.split(" :: ")[0] for k in base})
        rows.append(
            {
                "label": label,
                "baseline_log": base_log.resolve().relative_to(root).as_posix(),
                "merged_log": merged_log.resolve().relative_to(root).as_posix(),
                "baseline_tests": len(base),
                "merged_tests": len(merged),
                "baseline_tests_missing_after_merge": missing,
                "outcome_changed": changed,
                "new_tests_after_merge": len(new),
                "new_test_binaries_after_merge": new_binaries,
            }
        )
        print(
            f"{label}: baseline={len(base)} merged={len(merged)} missing={len(missing)} "
            f"changed={len(changed)} new={len(new)} new_binaries={len(new_binaries)}"
        )
    out = {
        "schema": "vigilode.loop-l1.trial-merge.test-set-compare.v1",
        "generator": "research/production_loop_20261116/trial_merge/compare_test_sets.py",
        "comparisons": rows,
    }
    out_path.write_text(json.dumps(out, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
