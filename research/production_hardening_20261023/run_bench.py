#!/usr/bin/env python3
"""Local paired DOPRI5 observation-policy experiment; never accesses NCP.

Default: 72 configurations, one warmup plus three timed repetitions. Existing
external-solver results are reused separately, never pooled into these timings.
"""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import random
import subprocess
import sys
import tarfile
import time

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MODELS = [("decay", 1), ("logistic", 1), ("driven", 1), ("rapid", 2),
          ("damped", 2), ("nonnormal", 2), ("large-vector", 4096),
          ("large-vector", 16384)]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def write_json(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temporary.replace(path)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--budget-seconds", type=float, default=900.0)
    parser.add_argument("--per-run-timeout", type=float, default=60.0)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--seed", type=int, default=20261023)
    parser.add_argument("--cases", help="Comma-separated registered subset; default all")
    parser.add_argument("--cpu-affinity", help="Comma-separated permitted logical CPU IDs")
    args = parser.parse_args()
    if not (0 < args.budget_seconds <= 900 and args.per_run_timeout > 0 and args.repeats >= 3):
        parser.error("budget in (0,900], positive timeout, and at least three timed repeats required")
    binary = pathlib.Path(args.binary).resolve()
    if not binary.is_file():
        parser.error("build the frozen measurement binary first")
    source = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if source != args.source_commit:
        parser.error(f"checkout HEAD {source} differs from requested source commit")
    tracked = ["Cargo.toml", "Cargo.lock", "crates", str(HERE.relative_to(ROOT))]
    if subprocess.run(["git", "diff", "--quiet", source, "--", *tracked], cwd=ROOT).returncode:
        parser.error("tracked code/research differs from frozen source; publish a new source before measuring")
    if args.cpu_affinity:
        os.sched_setaffinity(0, {int(v) for v in args.cpu_affinity.split(",")})
    folder = pathlib.Path(args.out).resolve()
    folder.mkdir(parents=True, exist_ok=False)
    (folder / "raw").mkdir()
    (folder / ".gitignore").write_text("/raw/\n")
    cases = set(args.cases.split(",")) if args.cases else {name for name, _ in MODELS}
    if not cases <= {name for name, _ in MODELS}:
        parser.error("unknown case")
    configurations, groups = [], []
    for case, n in MODELS:
        if case not in cases:
            continue
        for rtol in (1e-4, 1e-6, 1e-8):
            observations = 51 if n >= 4096 else 501
            group = []
            for mode, count in (("landing", observations), ("dense", observations), ("dense", 5)):
                cfg = {"id": len(configurations), "case": case, "dimension": n,
                       "mode": mode, "observations": count, "rtol": rtol, "atol": rtol * 1e-3}
                configurations.append(cfg)
                group.append(cfg["id"])
            groups.append(group)
    binary_before = digest(binary)
    tree = subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=ROOT, text=True).strip()
    plan = {"schema": "vigilode.production-hardening-campaign.v1", "source_commit": source,
            "source_tree": tree, "binary": str(binary), "binary_sha256": binary_before,
            "profile": "measurement", "compiler_options": "opt-level=3,lto=false,codegen-units=16",
            "ncp_used": False, "host": platform.node(), "platform": platform.platform(),
            "python": sys.version, "logical_cpus": os.cpu_count(),
            "affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
            "cgroup_cpu_max": pathlib.Path("/sys/fs/cgroup/cpu.max").read_text().strip()
            if pathlib.Path("/sys/fs/cgroup/cpu.max").exists() else None,
            "budget_seconds": args.budget_seconds, "per_run_timeout": args.per_run_timeout,
            "warmups_per_configuration": 1, "timed_repeats": args.repeats, "seed": args.seed,
            "order": "shuffle problem/tolerance groups each round, then shuffle observation modes within group",
            "timing_scope": "problem/state/options/workspace construction and solve; schedule construction/reference/JSON excluded",
            "error_metric": "max abs(error)/max(1,abs(initial),abs(exact)); all saved components and times",
            "accuracy_caps": [1e-4, 1e-6, 1e-8], "configurations": configurations,
            "holdout_cases": ["logistic", "rapid"], "external_comparators": "published cohort reused separately"}
    write_json(folder / "PLAN.json", plan)
    receipts = []
    started = time.monotonic()
    rng = random.Random(args.seed)
    for phase, repeat in [("warmup", -1)] + [("timed", i) for i in range(args.repeats)]:
        order = groups.copy()
        rng.shuffle(order)
        for group in order:
            group = group.copy()
            rng.shuffle(group)
            for idx in group:
                cfg = configurations[idx]
                stem = f"{idx:03d}_{phase}_{max(0, repeat)}"
                stdout = folder / "raw" / f"{stem}.stdout.json"
                stderr = folder / "raw" / f"{stem}.stderr.log"
                cmd = [str(binary), "--case", cfg["case"], "--mode", cfg["mode"],
                       "--dimension", str(cfg["dimension"]), "--observations", str(cfg["observations"]),
                       "--rtol", str(cfg["rtol"]), "--source-commit", source]
                row = {**cfg, "phase": phase, "repeat": repeat, "command": cmd,
                       "stdout": str(stdout.relative_to(folder)), "stderr": str(stderr.relative_to(folder))}
                remaining = args.budget_seconds - (time.monotonic() - started)
                if remaining <= 0:
                    row.update(status="NOT_RUN_BUDGET", exit_code=None, elapsed_process_seconds=0.0)
                else:
                    before = time.monotonic()
                    try:
                        with stdout.open("w") as out, stderr.open("w") as err:
                            run = subprocess.run(cmd, stdout=out, stderr=err,
                                                 timeout=min(args.per_run_timeout, remaining), check=False)
                        row.update(status="COMPLETED" if run.returncode == 0 else "PROCESS_FAILED",
                                   exit_code=run.returncode)
                    except subprocess.TimeoutExpired:
                        row.update(status="TIMEOUT", exit_code=None)
                    except OSError as error:
                        stderr.write_text(str(error) + "\n")
                        row.update(status="LAUNCH_FAILED", exit_code=None)
                    row["elapsed_process_seconds"] = time.monotonic() - before
                    for key, path in (("stdout", stdout), ("stderr", stderr)):
                        if path.exists():
                            row[key + "_sha256"] = digest(path)
                receipts.append(row)
                write_json(folder / "RECEIPTS.json", receipts)
                print(json.dumps({k: row[k] for k in ("id", "case", "mode", "observations", "phase", "repeat", "status")}), flush=True)
    binary_after = digest(binary)
    manifest = [{"path": str(p.relative_to(folder)), "bytes": p.stat().st_size, "sha256": digest(p)}
                for p in sorted((folder / "raw").iterdir())]
    write_json(folder / "RAW_MANIFEST.json", manifest)
    with tarfile.open(folder / "RAW_OUTPUTS.tar.gz", "w:gz") as archive:
        for p in sorted((folder / "raw").iterdir()):
            archive.add(p, arcname=str(p.relative_to(folder)), recursive=False)
    campaign = {"source_commit": source, "source_tree": tree, "ncp_used": False,
                "elapsed_seconds_including_packaging": time.monotonic() - started,
                "planned_configurations": len(configurations), "planned_runs": len(configurations) * (1 + args.repeats),
                "completed_processes": sum(r["status"] == "COMPLETED" for r in receipts),
                "receipt_count": len(receipts), "binary_sha256_before": binary_before,
                "binary_sha256_after": binary_after, "binary_unchanged": binary_before == binary_after,
                "raw_archive_sha256": digest(folder / "RAW_OUTPUTS.tar.gz"),
                "numerical_success_and_error_caps_are_separate": True}
    write_json(folder / "CAMPAIGN.json", campaign)
    if binary_before != binary_after:
        return 2
    return subprocess.run([sys.executable, str(HERE / "analyze_bench.py"), str(folder)], check=False).returncode

if __name__ == "__main__":
    raise SystemExit(main())
