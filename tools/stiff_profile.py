#!/usr/bin/env python3
"""Profile of the RODAS5P step path against Hairer's RODAS (research node
research/stiff_rodas5p_profile_20261002; see its PREREGISTRATION.md).

For each workload (problem, rtol 1e-6):

1. callgrind on `rodas5p stiff-profile-run` with 1 and with 2 repetitions; the difference is
   exactly one integration (startup and problem construction cancel).  Every instruction
   is attributed to the source file of its innermost inlined frame (callgrind fi/fe records)
   and the file to a preregistered category.
2. the same for the native driver (`native_stiff run <problem> hairer-rodas 1e-6 k 0`),
   attributed by function name.
3. perf cpu-clock sampling of the RODAS5P workload repeated, by symbol (a wall-time cross-check
   of the instruction profile; this VM exposes no hardware counters).

Writes PROFILE.json and the gzipped callgrind outputs.
"""

from __future__ import annotations

import argparse
import collections
import gzip
import json
import os
import re
import shutil
import subprocess
from pathlib import Path

SCHEMA = "vigilode-stiff-rodas5p-profile-v1"
RTOL = 1.0e-6
WORKLOADS = (("hires", 2000), ("van-der-pol-mu1000", 1500), ("brusselator-1d-200", 10))

# Preregistered categories, by source path of the innermost inlined frame (first match).
RUST_CATEGORIES = (
    ("lu-faer", re.compile(r"/faer[-_/]|/faer-traits|/pulp|/gemm|/dyn-stack|/reborrow|/equator")),
    ("dense-matrix-wrapper", re.compile(r"rodas5p-core/src/matrix\.rs$")),
    ("shifted-operator-and-matvec", re.compile(r"rodas5p-core/src/operator\.rs$")),
    ("problem-user-code", re.compile(r"rodas5p-cli/src/stiff_benchmark\.rs$|rodas5p-integrators/src/problems\.rs$")),
    ("problem-wrapper", re.compile(r"rodas5p-integrators/src/problem\.rs$")),
    ("stage-assembly", re.compile(r"rodas5p-integrators/src/sequential\.rs$")),
    ("adaptive-control-and-output", re.compile(r"rodas5p-integrators/src/(integrate|adaptive|output|dense_output\w*)\.rs$")),
    ("coefficients", re.compile(r"rodas5p-core/src/coefficients\.rs$")),
    ("norms", re.compile(r"rodas5p-core/src/norms\.rs$")),
    ("work-counters", re.compile(r"rodas5p-core/src/work\.rs$")),
    ("rust-std-alloc-iter", re.compile(r"/library/(alloc|core|std)/")),
    ("libc-allocator-and-memory", re.compile(r"^obj:.*libc\.so")),
)

NATIVE_CATEGORIES = (
    ("lu-decsol", re.compile(r"^(dec|sol|decb|solb|decc|solc|dech|solh|dechc|solhc|decbc|solbc)_$")),
    ("integrator-core", re.compile(r"^(rodas_|roscor_|rocoe_|contro_|slvrod_|decomr_|decomc_|estrav_|estrad_)$")),
    ("problem-user-code", re.compile(r"^(robertson|hires|vdp|bruss)_(rhs|jac)$")),
    ("problem-wrapper", re.compile(r"^h_(fcn|jac)$")),
    ("libc-allocator-and-memory", re.compile(r"^obj:.*libc\.so")),
    ("fortran-runtime", re.compile(r"^obj:.*libgfortran")),
    ("libm", re.compile(r"^obj:.*libm\.so")),
)


def parse_callgrind(path: Path):
    """Self Ir by (file, line), by function and by object, from a callgrind output.

    Cost lines that follow a `calls=` line are inclusive costs of the call and are skipped.
    Inlined code is attributed to its `fi=`/`fe=` file.  Position compression (+n, -n, *)
    is resolved for the line number.
    """
    names = {"fl": {}, "fn": {}, "ob": {}}
    cur = {"ob": "???", "fl": "???", "file": "???", "fn": "???"}
    by_file_line = collections.Counter()
    by_fn = collections.Counter()
    by_ob = collections.Counter()
    by_ob_fn = collections.Counter()
    total = 0
    skip_next = False
    last_line = 0
    events = None
    summary = None

    def resolve(kind, value):
        m = re.match(r"\((\d+)\)(?:\s+(.*))?$", value.strip())
        if not m:
            return value.strip()
        table = names["fl" if kind in ("fl", "fi", "fe") else kind]
        if m.group(2) is not None:
            table[m.group(1)] = m.group(2)
        return table.get(m.group(1), "?")

    with open(path) as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            if not line:
                continue
            if line.startswith("events:"):
                events = line.split()[1:]
                continue
            if line.startswith(("summary:", "totals:")):
                summary = int(line.split()[1])
                continue
            key, _, value = line.partition("=")
            if key in ("ob", "cob"):
                name = resolve("ob", value)
                if key == "ob":
                    cur["ob"] = name
                continue
            if key in ("fl", "fi", "fe", "cfi", "cfl"):
                name = resolve("fl", value)
                if key == "fl":
                    cur["fl"] = name
                    cur["file"] = name
                elif key in ("fi", "fe"):
                    cur["file"] = name
                continue
            if key in ("fn", "cfn"):
                name = resolve("fn", value)
                if key == "fn":
                    cur["fn"] = name
                    cur["file"] = cur["fl"]
                continue
            if key == "calls":
                skip_next = True
                continue
            if line[0].isdigit() or line[0] in "+-*":
                parts = line.split()
                pos = parts[0]
                if pos == "*":
                    lineno = last_line
                elif pos[0] in "+-":
                    lineno = last_line + int(pos)
                else:
                    lineno = int(pos)
                last_line = lineno
                if skip_next:
                    skip_next = False
                    continue
                ir = int(parts[1]) if len(parts) > 1 else 0
                total += ir
                file = cur["file"]
                if file in ("???", "?"):
                    file = "obj:" + cur["ob"]
                by_file_line[(file, lineno)] += ir
                by_fn[cur["fn"]] += ir
                by_ob[cur["ob"]] += ir
                by_ob_fn[(cur["ob"], cur["fn"])] += ir
    assert events is None or events[0] == "Ir", events
    # Self costs must add up to callgrind's own total.
    assert summary is None or summary == total, (path, summary, total)
    return {"total": total, "by_file_line": by_file_line, "by_fn": by_fn, "by_ob": by_ob,
            "by_ob_fn": by_ob_fn}


def difference(two, one):
    out = {"total": two["total"] - one["total"]}
    for key in ("by_file_line", "by_fn", "by_ob", "by_ob_fn"):
        diff = collections.Counter(two[key])
        diff.subtract(one[key])
        out[key] = collections.Counter({k: v for k, v in diff.items() if v != 0})
    return out


def categorize_rust(profile):
    cats = collections.Counter()
    by_file = collections.Counter()
    for (file, _), ir in profile["by_file_line"].items():
        by_file[file] += ir
    for file, ir in by_file.items():
        for name, pattern in RUST_CATEGORIES:
            if pattern.search(file):
                cats[name] += ir
                break
        else:
            cats["other"] += ir
    return cats, by_file


def categorize_native(profile):
    """By function name; code of a shared library without a matching name goes by object."""
    cats = collections.Counter()
    for (ob, fn), ir in profile["by_ob_fn"].items():
        for name, pattern in NATIVE_CATEGORIES:
            if pattern.search(fn) or pattern.search("obj:" + ob):
                cats[name] += ir
                break
        else:
            cats["other"] += ir
    return cats


def shares(counter, total):
    return {k: {"ir": v, "share": v / total} for k, v in sorted(counter.items(), key=lambda kv: -kv[1])}


def callgrind(cmd, out: Path):
    subprocess.run(["valgrind", "--tool=callgrind", f"--callgrind-out-file={out}", "--dump-line=yes",
                    "--dump-instr=no", "--compress-strings=yes", "--compress-pos=yes", *cmd],
                   check=True, capture_output=True, text=True)
    return parse_callgrind(out)


def run_json(cmd):
    out = subprocess.run(cmd, check=True, capture_output=True, text=True).stdout
    return json.loads([l for l in out.splitlines() if l.startswith("{")][-1])


def gzip_into(src: Path, dst_dir: Path):
    dst = dst_dir / (src.name + ".gz")
    with open(src, "rb") as fin, gzip.GzipFile(dst, "wb", compresslevel=9, mtime=0) as fout:
        shutil.copyfileobj(fin, fout)
    return dst


def perf_symbols(perf, cmd, out: Path):
    subprocess.run([perf, "record", "-e", "cpu-clock", "-F", "4999", "-o", str(out), "--", *cmd],
                   check=True, capture_output=True, text=True)
    report = subprocess.run([perf, "report", "-i", str(out), "--stdio", "--no-children", "--sort", "sym",
                             "--percent-limit", "0.5"], check=True, capture_output=True, text=True).stdout
    samples = re.search(r"of event 'cpu-clock'\s*\n#\s*Event count \(approx\.\):\s*(\d+)", report)
    count = re.search(r"# Samples: (\S+)", report)
    rows = []
    for line in report.splitlines():
        m = re.match(r"\s+([\d.]+)%\s+\[\.\]\s+(.*)$", line)
        if m:
            rows.append({"percent": float(m.group(1)), "symbol": m.group(2).strip()})
    return {"samples": count.group(1) if count else None, "symbols": rows[:25]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rodas5p", required=True, help="RODAS5P binary built with debug line tables")
    parser.add_argument("--driver", required=True, help="native_stiff driver")
    parser.add_argument("--perf", required=True)
    parser.add_argument("--benchmark", required=True, type=Path, help="ANALYSIS.json of the native benchmark")
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--scratch", required=True, type=Path)
    args = parser.parse_args()
    out_json = args.output_dir / "PROFILE.json"
    if out_json.exists():
        raise SystemExit(f"immutable output already exists: {out_json}")
    raw_dir = args.output_dir / "callgrind"
    raw_dir.mkdir(parents=True, exist_ok=False)
    args.scratch.mkdir(parents=True, exist_ok=True)
    env_note = {k: os.environ.get(k) for k in ("RAYON_NUM_THREADS", "OPENBLAS_NUM_THREADS")}
    bench_rows = json.loads(args.benchmark.read_text())["rows"]
    report = {"schema": SCHEMA, "rtol": RTOL, "environment": env_note, "workloads": []}
    for problem, perf_reps in WORKLOADS:
        bench = {r["arm"]: r for r in bench_rows if r["problem"] == problem and r["rtol"] == RTOL}
        entry = {"problem": problem}
        rust_cmd = lambda k: [args.rodas5p, "stiff-profile-run", "--problem", problem, "--rtol", repr(RTOL),
                              "--repetitions", str(k)]
        native_cmd = lambda k: [args.driver, "run", problem, "hairer-rodas", repr(RTOL), str(k), "0"]
        work = run_json(rust_cmd(1))
        profiles = {}
        for k in (1, 2):
            path = args.scratch / f"callgrind.rodas5p.{problem}.{k}"
            profiles[k] = callgrind(rust_cmd(k), path)
            gzip_into(path, raw_dir)
        again = callgrind(rust_cmd(1), args.scratch / f"callgrind.rodas5p.{problem}.1b")
        run = difference(profiles[2], profiles[1])
        cats, by_file = categorize_rust(run)
        attempts = work["attempts"]
        b = bench["rodas5p"]
        entry["rodas5p"] = {
            "work": work,
            "matches_benchmark": work["accepted_steps"] == b["accepted_steps"]
            and work["counters"]["direct_factorizations"] == b["counters"]["direct_factorizations"]
            and work["counters"]["rhs_evaluations"] == b["counters"]["rhs_evaluations"],
            "callgrind_deterministic": again["total"] == profiles[1]["total"],
            "ir_per_run": run["total"],
            "ir_per_attempt": run["total"] / attempts,
            "categories": shares(cats, run["total"]),
            "top_files": [{"file": f, "ir": v, "share": v / run["total"]} for f, v in by_file.most_common(15)],
            "top_lines": [{"file": f, "line": ln, "ir": v, "share": v / run["total"]}
                          for (f, ln), v in run["by_file_line"].most_common(30)],
            "top_functions": [{"function": f, "ir": v, "share": v / run["total"]}
                              for f, v in run["by_fn"].most_common(15)],
        }
        nprof = {}
        for k in (1, 2):
            path = args.scratch / f"callgrind.hairer-rodas.{problem}.{k}"
            nprof[k] = callgrind(native_cmd(k), path)
            gzip_into(path, raw_dir)
        nrun = difference(nprof[2], nprof[1])
        nwork = bench["hairer-rodas"]
        nattempts = nwork["accepted_steps"] + nwork["rejected_steps"]
        entry["hairer-rodas"] = {
            "work": {k: nwork[k] for k in ("accepted_steps", "rejected_steps", "counters")},
            "ir_per_run": nrun["total"],
            "ir_per_attempt": nrun["total"] / nattempts,
            "categories": shares(categorize_native(nrun), nrun["total"]),
            "top_functions": [{"function": f, "ir": v, "share": v / nrun["total"]}
                              for f, v in nrun["by_fn"].most_common(15)],
        }
        entry["perf_rodas5p"] = perf_symbols(args.perf, rust_cmd(perf_reps),
                                             args.scratch / f"perf.rodas5p.{problem}.data")
        entry["perf_rodas5p"]["repetitions"] = perf_reps
        print(f"{problem}: rodas5p {entry['rodas5p']['ir_per_attempt']:.4g} Ir/attempt, "
              f"hairer-rodas {entry['hairer-rodas']['ir_per_attempt']:.4g} Ir/attempt")
        report["workloads"].append(entry)
    gate = {
        "profiled_runs_match_benchmark": all(w["rodas5p"]["matches_benchmark"] for w in report["workloads"]),
        "callgrind_deterministic": all(w["rodas5p"]["callgrind_deterministic"] for w in report["workloads"]),
        "rust_attribution_at_least_90_percent": all(
            1.0 - w["rodas5p"]["categories"].get("other", {"share": 0.0})["share"] >= 0.90
            for w in report["workloads"]),
        "native_attribution_at_least_90_percent": all(
            1.0 - w["hairer-rodas"]["categories"].get("other", {"share": 0.0})["share"] >= 0.90
            for w in report["workloads"]),
    }
    report["validity_gate"] = gate
    report["verdict"] = "PASS" if all(gate.values()) else "FAIL"
    out_json.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"verdict": report["verdict"], "gate": gate}))


if __name__ == "__main__":
    main()
