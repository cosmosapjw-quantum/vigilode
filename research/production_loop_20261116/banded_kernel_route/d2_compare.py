#!/usr/bin/env python3
"""Discriminator D2 of node V22: the golden file of the banded facade runs against the file of
commit A, which was recorded before the facade was edited.

usage: d2_compare.py [<commit A>]   (default 66c558a0)

Prints which fields differ per record and the bands whose records changed. Exit 1 if a record
differs in a field other than the two labels (driver_id, entry_point), if the records do not
pair by case, or if the set of changed bands is not the set with both half widths >= W, where W
is read from crates/vigilode/src/solve.rs.
"""
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
GOLDEN = "crates/vigilode/tests/golden/banded_kernel_route/wide_band.json"
LABELS = {"driver_id", "entry_point"}


def main():
    commit = sys.argv[1] if len(sys.argv) > 1 else "66c558a0"
    old = json.loads(subprocess.run(["git", "-C", ROOT, "show", f"{commit}:{GOLDEN}"],
                                    capture_output=True, check=True).stdout)
    with open(os.path.join(ROOT, GOLDEN)) as f:
        new = json.load(f)
    with open(os.path.join(ROOT, "crates/vigilode/src/solve.rs")) as f:
        width = int(re.search(r"BANDED_SLICES_MIN_HALF_WIDTH: usize = (\d+);", f.read()).group(1))
    old_by, new_by = {r["case"]: r for r in old}, {r["case"]: r for r in new}
    ok = len(old) == len(new) == len(old_by) == len(new_by) and set(old_by) == set(new_by)
    sets, changed, unchanged = {}, set(), set()
    for case, a in old_by.items():
        b = new_by.get(case, {})
        fields = sorted(k for k in set(a) | set(b) if a.get(k) != b.get(k))
        key = " + ".join(fields) if fields else "none"
        sets[key] = sets.get(key, 0) + 1
        band = (int(a["lower"]), int(a["upper"]))
        (changed if fields else unchanged).add(band)
        ok = ok and set(fields) <= LABELS and (not fields or set(fields) == LABELS)
    expected = {band for band in changed | unchanged if min(band) >= width}
    ok = ok and changed == expected and not (changed & unchanged)
    print(f"D2_GOLDEN records {len(new)} against commit {commit} (recorded before the facade edit): "
          f"changed field sets {sets}")
    print(f"D2_GOLDEN width W = {width}; bands whose records changed (labels only): {sorted(changed)}")
    print(f"D2_GOLDEN bands whose records are unchanged: {sorted(unchanged)}")
    print("D2_GOLDEN holds:", ok)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
