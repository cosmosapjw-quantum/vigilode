#!/usr/bin/env python3
"""Deterministic instruction profile of `rodas5p stiff-profile-run` arms (speed research, 2026-10-05).

Protocol (as in L-0030/L-0032/L-0033/L-0041): callgrind on one process with 1 and with 2
repetitions of the integration; the difference is one integration, with process start-up and
problem construction cancelled. Instructions per attempted step = difference / attempts, where
`attempts` (accepted + rejected) comes from the 1-repetition JSON line. The 1-repetition run is
repeated once as a determinism check. No perf, no wall clock: counted instructions only.

Unlike tools/stiff_profile.py this tool takes any arm and problem the CLI knows, records the
self cost by function, by source file and by (file, line) from a line-tables build
(`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only cargo build --release -p rodas5p-cli --locked`),
and attributes every instruction to a category by source file or object with a coverage figure
(no gate here; gates belong to the node that calls this tool). With `--cache-sim` the callgrind
cache and branch simulators run too and their event totals are recorded for the difference
(Ir, Dr, Dw, I1mr, D1mr, D1mw, ILmr, DLmr, DLmw, Bc, Bcm, Bi, Bim); they are reported, never
gated, because the simulated cache is a model and not this host.

The output file is immutable: the tool refuses to overwrite it.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path

SCHEMA = "vigilode-speed-profile-v1"
HERE = Path(__file__).resolve().parent

CATEGORIES = [
    # (name, pattern on the source file path or "obj:<object>")
    ("driver-fast", re.compile(r"rodas5p-integrators/src/rodas5p_fast\.rs$")),
    ("driver-fast-small", re.compile(r"rodas5p-integrators/src/rodas5p_fast_small\.rs$")),
    ("driver-fast-banded", re.compile(r"rodas5p-integrators/src/rodas5p_fast_banded\.rs$")),
    ("driver-mf-fast", re.compile(r"rodas5p-integrators/src/rodas5p_matrix_free_fast\.rs$")),
    ("driver-sequential", re.compile(r"rodas5p-integrators/src/(sequential|integrate|stage_batch)\.rs$")),
    ("output-and-clock", re.compile(r"rodas5p-integrators/src/(output|dense_output.*)\.rs$")),
    ("adaptive-control", re.compile(r"rodas5p-integrators/src/adaptive\.rs$")),
    ("problem-wrapper", re.compile(r"rodas5p-integrators/src/problem\.rs$")),
    ("problem-user-code", re.compile(r"(rodas5p-cli/src/(stiff_benchmark|problems)\.rs|rodas5p-integrators/src/problems\.rs)$")),
    ("krylov", re.compile(r"rodas5p-krylov/src/")),
    ("core-matrix-operator", re.compile(r"rodas5p-core/src/(matrix|operator|banded.*)\.rs$")),
    ("core-other", re.compile(r"rodas5p-core/src/")),
    ("integrators-other", re.compile(r"rodas5p-integrators/src/")),
    ("cli-other", re.compile(r"rodas5p-cli/src/")),
    ("faer", re.compile(r"/(faer|pulp|gemm|dyn-stack|reborrow|equator|nano-gemm)[^/]*/")),
    ("rust-std", re.compile(r"/rustc/|/library/(core|alloc|std)/")),
    ("glibc-malloc", re.compile(r"^obj:.*libc\.so|/malloc/|/malloc\.c|_int_free|_int_malloc")),
    ("glibc-libm", re.compile(r"/sysdeps/ieee754/|libm\.so|/math/")),
    ("glibc-string", re.compile(r"/sysdeps/x86_64/multiarch/|/string/")),
    ("glibc-other", re.compile(r"/glibc|/csu/|/elf/|/nptl/|/misc/|/stdlib/|/io/|/time/")),
]


def load_stiff_profile():
    spec = importlib.util.spec_from_file_location("stiff_profile", HERE / "stiff_profile.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def event_totals(path: Path) -> dict:
    """All event totals of a callgrind output (the `summary:`/`totals:` line)."""
    events = None
    totals = None
    with open(path) as handle:
        for line in handle:
            if line.startswith("events:"):
                events = line.split()[1:]
            elif line.startswith(("summary:", "totals:")):
                totals = [int(x) for x in line.split()[1:]]
    if events is None or totals is None:
        raise RuntimeError(f"no events/totals in {path}")
    return dict(zip(events, totals))


FUNCTION_CATEGORIES = [
    # By outlined symbol: inlined std/iterator code is charged to the Rust function it was
    # inlined into. A generic driver monomorphized into a CLI function (e.g.
    # `rodas5p::stiff_benchmark::run_small`) is charged to that CLI function here and to
    # the driver's source file in the file view; the two views bracket the truth.
    ("driver-fast", re.compile(r"rodas5p_integrators::rodas5p_fast::")),
    ("driver-fast-small", re.compile(r"rodas5p_integrators::rodas5p_fast_small::")),
    ("driver-fast-banded", re.compile(r"rodas5p_integrators::rodas5p_fast_banded::")),
    ("driver-mf-fast", re.compile(r"rodas5p_integrators::rodas5p_matrix_free_fast::")),
    ("driver-sequential", re.compile(r"rodas5p_integrators::(sequential|integrate|stage_batch)::")),
    ("output-and-clock", re.compile(r"rodas5p_integrators::(output|dense_output\w*)::")),
    ("adaptive-control", re.compile(r"rodas5p_integrators::adaptive::")),
    ("problem-wrapper", re.compile(r"rodas5p_integrators::problem::")),
    ("problem-user-code", re.compile(r"rodas5p::(stiff_benchmark|problems)::|rodas5p_integrators::problems::")),
    ("krylov", re.compile(r"rodas5p_krylov::")),
    ("core-matrix-operator", re.compile(r"rodas5p_core::(matrix|operator|banded\w*)::")),
    ("core-other", re.compile(r"rodas5p_core::")),
    ("integrators-other", re.compile(r"rodas5p_integrators::")),
    ("cli-other", re.compile(r"^rodas5p::")),
    ("faer", re.compile(r"^(faer|pulp|gemm|dyn_stack|nano_gemm)")),
    ("rust-std", re.compile(r"^(core|alloc|std)::|^<(core|alloc|std)::")),
    ("glibc-malloc", re.compile(r"^(_int_free|_int_malloc|malloc|free|cfree|realloc|calloc|unlink_chunk|malloc_consolidate|tcache|__libc_malloc|__libc_free|sysmalloc|arena)")),
    ("glibc-libm", re.compile(r"^(__ieee754_|pow|exp|log|sqrt|__pow|__exp|__log)")),
    ("glibc-string", re.compile(r"^(__mem|__str|mem|str)(cpy|move|set|cmp|len|chr)")),
]


def _attribute(weights, categories):
    cats = collections.Counter()
    for key, ir in weights.items():
        name = "other"
        for cat, pattern in categories:
            if pattern.search(key):
                name = cat
                break
        cats[name] += ir
    return cats


CALL_WATCH = re.compile(r"rodas5p_integrators::(output|adaptive)::|validate|propose_factor|land_capped|step_to|limit_step|limit_landed_step|accept")


def parse_calls(path: Path):
    """Call counts by callee and by (caller, callee) from the `calls=` records of a
    callgrind output (name compression resolved as in `parse_callgrind`)."""
    names = {}
    caller = "???"
    callee = "???"
    by_callee = collections.Counter()
    by_pair = collections.Counter()

    def resolve(value):
        m = re.match(r"\((\d+)\)(?:\s+(.*))?$", value.strip())
        if not m:
            return value.strip()
        if m.group(2) is not None:
            names[m.group(1)] = m.group(2)
        return names.get(m.group(1), "?")

    with open(path) as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            if line.startswith("fn="):
                caller = resolve(line[3:])
            elif line.startswith("cfn="):
                callee = resolve(line[4:])
            elif line.startswith("calls="):
                count = int(line.split()[0][6:])
                by_callee[callee] += count
                by_pair[(caller, callee)] += count
    return by_callee, by_pair


def call_difference(path2: Path, path1: Path, attempts: int):
    """Calls per integration (2-rep minus 1-rep) and per attempt, for the watched
    functions and the most-called callees."""
    callee2, pair2 = parse_calls(path2)
    callee1, pair1 = parse_calls(path1)
    callee = collections.Counter(callee2)
    callee.subtract(callee1)
    pair = collections.Counter(pair2)
    pair.subtract(pair1)
    watched = {f: {"calls": c, "per_attempt": c / attempts}
               for f, c in sorted(callee.items(), key=lambda kv: -kv[1]) if c != 0 and CALL_WATCH.search(f)}
    top = [{"function": f, "calls": c, "per_attempt": c / attempts}
           for f, c in sorted(callee.items(), key=lambda kv: -kv[1])[:40] if c != 0]
    pairs = [{"caller": a, "callee": b, "calls": c, "per_attempt": c / attempts}
             for (a, b), c in sorted(pair.items(), key=lambda kv: -kv[1])
             if c != 0 and CALL_WATCH.search(b)]
    return {"watched_callees": watched, "top_callees": top, "watched_pairs": pairs}


def categorize(profile, repo_root: Path):
    """Two attributions: by source file of the innermost inlined frame and by outlined symbol."""
    by_file = collections.Counter()
    for (file, _), ir in profile["by_file_line"].items():
        by_file[file] += ir
    total = profile["total"]
    by_file_cats = _attribute(by_file, CATEGORIES)
    by_fn_cats = _attribute(profile["by_fn"], FUNCTION_CATEGORIES)

    def shares(cats):
        return {k: {"ir": v, "share": v / total if total else 0.0}
                for k, v in sorted(cats.items(), key=lambda kv: -kv[1])}

    named_file = (total - by_file_cats.get("other", 0)) / total if total else 0.0
    named_fn = (total - by_fn_cats.get("other", 0)) / total if total else 0.0
    return shares(by_file_cats), named_file, shares(by_fn_cats), named_fn


def rel(path: str, repo_root: Path) -> str:
    root = str(repo_root)
    return path[len(root) + 1:] if path.startswith(root + "/") else path


def profile_arm(stiff, args, arm: str, problem: str, repo_root: Path) -> dict:
    def cmd(k: int):
        return [args.rodas5p, "stiff-profile-run", "--problem", problem, "--arm", arm,
                "--rtol", repr(args.rtol), "--repetitions", str(k)]

    extra = ["--cache-sim=yes", "--branch-sim=yes"] if args.cache_sim else []
    work = stiff.run_json(cmd(1))
    if not work.get("success", False):
        return {"arm": arm, "problem": problem, "success": False, "work": work}
    outs = {}
    runs = {}
    for tag, k in (("1", 1), ("2", 2), ("1b", 1)):
        out = args.scratch / f"callgrind.{arm}.{problem}.{tag}"
        subprocess.run(["valgrind", "--tool=callgrind", f"--callgrind-out-file={out}", "--dump-line=yes",
                        "--dump-instr=no", "--compress-strings=yes", "--compress-pos=yes", *extra, *cmd(k)],
                       check=True, capture_output=True, text=True)
        outs[tag] = out
        runs[tag] = stiff.parse_callgrind(out)
    diff = stiff.difference(runs["2"], runs["1"])
    attempts = work["attempts"]
    ir = diff["total"]
    top_fn = sorted(diff["by_fn"].items(), key=lambda kv: -kv[1])[: args.top]
    top_lines = sorted(diff["by_file_line"].items(), key=lambda kv: -kv[1])[: args.top]
    by_file = collections.Counter()
    for (file, _), v in diff["by_file_line"].items():
        by_file[file] += v
    top_files = sorted(by_file.items(), key=lambda kv: -kv[1])[: args.top]
    categories, coverage, fn_categories, fn_coverage = categorize(diff, repo_root)
    entry = {
        "arm": arm, "problem": problem, "rtol": args.rtol, "success": True,
        "attempts": attempts, "accepted_steps": work["accepted_steps"], "rejected_steps": work["rejected_steps"],
        "counters": work["counters"],
        "banded_work": work.get("banded_work"),
        "final_state_sha256": hashlib.sha256(json.dumps(work["final_state"]).encode()).hexdigest(),
        "ir_run1": runs["1"]["total"], "ir_run2": runs["2"]["total"], "ir_run1_repeat": runs["1b"]["total"],
        "callgrind_deterministic": runs["1"]["total"] == runs["1b"]["total"],
        "ir_per_run": ir, "ir_per_attempt": ir / attempts,
        "ir_per_accepted_step": ir / work["accepted_steps"] if work["accepted_steps"] else None,
        "categories_by_file": categories, "named_coverage_by_file": coverage,
        "categories_by_function": fn_categories, "named_coverage_by_function": fn_coverage,
        "top_functions": [{"function": f, "ir": v, "share": v / ir} for f, v in top_fn],
        "top_files": [{"file": rel(f, repo_root), "ir": v, "share": v / ir} for f, v in top_files],
        "top_lines": [{"file": rel(f, repo_root), "line": ln, "ir": v, "share": v / ir}
                      for (f, ln), v in top_lines],
    }
    entry["calls"] = call_difference(outs["2"], outs["1"], attempts)
    if args.cache_sim:
        t1, t2 = event_totals(outs["1"]), event_totals(outs["2"])
        entry["events_per_run"] = {e: t2[e] - t1[e] for e in t1}
        entry["events_per_attempt"] = {e: (t2[e] - t1[e]) / attempts for e in t1}
    if args.keep_callgrind:
        entry["callgrind_files"] = {tag: str(p) for tag, p in outs.items()}
    return entry


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--rodas5p", required=True, help="rodas5p CLI binary (release, line tables)")
    parser.add_argument("--arms", required=True, help="comma-separated stiff-profile-run arms")
    parser.add_argument("--problems", required=True, help="comma-separated benchmark problem ids")
    parser.add_argument("--rtol", type=float, default=1.0e-6)
    parser.add_argument("--scratch", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--top", type=int, default=30)
    parser.add_argument("--cache-sim", action="store_true", help="also record cache/branch simulator totals")
    parser.add_argument("--keep-callgrind", action="store_true", help="record the callgrind output paths")
    parser.add_argument("--label", default="", help="free text stored with the output (e.g. the commit)")
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    args.scratch.mkdir(parents=True, exist_ok=True)
    stiff = load_stiff_profile()
    repo_root = HERE.parent
    toolchain = subprocess.run(["valgrind", "--version"], capture_output=True, text=True, check=True).stdout.strip()
    entries = []
    for arm in args.arms.split(","):
        for problem in args.problems.split(","):
            entry = profile_arm(stiff, args, arm.strip(), problem.strip(), repo_root)
            entries.append(entry)
            if entry["success"]:
                print(f"{arm} {problem}: attempts {entry['attempts']}, Ir/attempt {entry['ir_per_attempt']:.1f}, "
                      f"deterministic {entry['callgrind_deterministic']}, named by file "
                      f"{entry['named_coverage_by_file']:.3f}, by function {entry['named_coverage_by_function']:.3f}",
                      flush=True)
            else:
                print(f"{arm} {problem}: FAILED {entry['work'].get('message', '')}", flush=True)
    report = {"schema": SCHEMA, "label": args.label, "valgrind": toolchain, "rtol": args.rtol,
              "binary_sha256": hashlib.sha256(Path(args.rodas5p).read_bytes()).hexdigest(),
              "protocol": "callgrind Ir of stiff-profile-run with 2 minus 1 repetitions, divided by attempts of the "
                          "1-repetition run; 1-repetition run repeated as determinism check; no wall clock",
              "cache_sim": bool(args.cache_sim), "entries": entries}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
