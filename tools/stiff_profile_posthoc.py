#!/usr/bin/env python3
"""Post-hoc analysis of the RODAS5P profile (research/stiff_rodas5p_profile_20261002).

Not preregistered.  The preregistered attribution failed its 90% coverage gate because two
cases were mapped wrongly: glibc in this container has line tables (./malloc/malloc.c,
sysdeps/...), and faer's gemm microkernels have no line tables.  This script reads the raw
callgrind files kept in the node and

1. recomputes the categories with those two corrections (glibc sources -> allocator and memory;
   unsymbolized code in the RODAS5P binary resolved with addr2line, gemm kernels -> LU);
2. attributes the inclusive cost of calls into the hot callees (dense matvec, the allocator,
   faer's factorization and solve, matrix construction) to their calling functions.

Writes POSTHOC.json.  Everything here is exploratory and labelled as such.
"""

from __future__ import annotations

import argparse
import collections
import gzip
import importlib.util
import json
import re
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCHEMA = "vigilode-stiff-rodas5p-profile-posthoc-v1"
GLIBC = re.compile(r"^\./(malloc|string|stdlib|libio)/|/sysdeps/|^obj:.*libc\.so")
HOT_CALLEES = {
    "dense-matvec": re.compile(r"DenseMatrix::matvec_into"),
    "allocator": re.compile(r"^(malloc|free|calloc|realloc|_int_malloc|_int_free|_mid_memalign|cfree)"),
    "faer-lu-factor": re.compile(r"faer::linalg::lu::"),
    "faer-solve": re.compile(r"faer::linalg::solvers::Solve::solve|triangular_solve"),
    "matrix-construction": re.compile(r"DenseMatrix::(new|zeros|combine|clone)|memset"),
}


def load_profile_tool():
    spec = importlib.util.spec_from_file_location("stiff_profile", HERE / "stiff_profile.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def parse_calls(path: Path):
    """Inclusive Ir of calls by (caller function, callee function)."""
    names = {"fn": {}}
    edges = collections.Counter()
    cur_fn, callee, pending = "?", None, False

    def resolve(value):
        m = re.match(r"\((\d+)\)(?:\s+(.*))?$", value.strip())
        if not m:
            return value.strip()
        if m.group(2) is not None:
            names["fn"][m.group(1)] = m.group(2)
        return names["fn"].get(m.group(1), "?")

    with gzip.open(path, "rt") as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            if line.startswith("fn="):
                cur_fn = resolve(line[3:])
            elif line.startswith("cfn="):
                callee = resolve(line[4:])
            elif line.startswith("calls="):
                pending = True
            elif pending and line and (line[0].isdigit() or line[0] in "+-*"):
                parts = line.split()
                edges[(cur_fn, callee)] += int(parts[1]) if len(parts) > 1 else 0
                pending = False
    return edges


def resolver(binary: str):
    cache = {}

    def name(address: str) -> str:
        if address not in cache:
            out = subprocess.run(["addr2line", "-f", "-C", "-e", binary, address],
                                 capture_output=True, text=True).stdout.splitlines()
            cache[address] = out[0] if out else address
        return cache[address]

    return name


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--node", required=True, type=Path)
    parser.add_argument("--rodas5p", required=True)
    args = parser.parse_args()
    out = args.node / "POSTHOC.json"
    if out.exists():
        raise SystemExit(f"immutable output already exists: {out}")
    tool = load_profile_tool()
    name_of = resolver(args.rodas5p)
    report = {"schema": SCHEMA, "preregistered": False, "workloads": []}
    profile = json.loads((args.node / "PROFILE.json").read_text())
    for workload in profile["workloads"]:
        problem = workload["problem"]
        raw = {}
        for k in (1, 2):
            gz = args.node / "callgrind" / f"callgrind.rodas5p.{problem}.{k}.gz"
            plain = Path(f"/tmp/posthoc.{problem}.{k}")
            plain.write_bytes(gzip.decompress(gz.read_bytes()))
            raw[k] = (tool.parse_callgrind(plain), parse_calls(gz))
            plain.unlink()
        run = tool.difference(raw[2][0], raw[1][0])
        total = run["total"]
        # 1. corrected categories
        cats = collections.Counter()
        resolved = collections.Counter()
        by_file = collections.Counter()
        for (file, _), ir in run["by_file_line"].items():
            by_file[file] += ir
        unsymbolized = collections.Counter()
        for (ob, fn), ir in run["by_ob_fn"].items():
            if ob.endswith("/rodas5p") and fn.startswith("0x"):
                unsymbolized[fn] += ir
        for fn, ir in unsymbolized.items():
            resolved[name_of(fn)] += ir
        binary_obj = next((f for f in by_file if f.startswith("obj:") and f.endswith("/rodas5p")), None)
        for file, ir in by_file.items():
            if file == binary_obj:
                continue
            if GLIBC.search(file):
                cats["libc-allocator-and-memory"] += ir
                continue
            for cat, pattern in tool.RUST_CATEGORIES:
                if pattern.search(file):
                    cats[cat] += ir
                    break
            else:
                cats["other"] += ir
        for fn, ir in resolved.items():
            cats["lu-faer" if fn.startswith("gemm_") else "other-unsymbolized"] += ir
        if binary_obj:
            leftover = by_file[binary_obj] - sum(resolved.values())
            if leftover:
                cats["other-unsymbolized"] += leftover
        # 2. inclusive cost of hot callees, by caller (one run = difference of 2 and 1 reps)
        edges = raw[2][1].copy()
        edges.subtract(raw[1][1])
        callers = {}
        for label, pattern in HOT_CALLEES.items():
            by_caller = collections.Counter()
            for (caller, callee), ir in edges.items():
                if ir and pattern.search(callee) and not pattern.search(caller):
                    by_caller[caller] += ir
            callers[label] = {
                "inclusive_ir": sum(by_caller.values()),
                "share": sum(by_caller.values()) / total,
                "top_callers": [{"caller": c, "ir": v, "share": v / total} for c, v in by_caller.most_common(8)],
            }
        report["workloads"].append({
            "problem": problem,
            "ir_per_run": total,
            "corrected_categories": tool.shares(cats, total),
            "unsymbolized_resolved": [{"function": f, "ir": v, "share": v / total}
                                      for f, v in resolved.most_common(8)],
            "hot_callees_by_caller": callers,
        })
        print(problem, {k: round(v["share"] * 100, 1) for k, v in tool.shares(cats, total).items()})
    out.write_text(json.dumps(report, indent=1) + "\n")


if __name__ == "__main__":
    main()
