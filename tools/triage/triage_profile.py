#!/usr/bin/env python3
"""Counted-instruction cost of component-triage cells (callgrind on `rodas5p triage-profile-run`).

Protocol (CT-HARNESS v1, section 1; as tools/speed_profile.py): one callgrind process per repetition count
K in (0, 1, 2) plus a second K = 1 process (`1b`). With Ir(K) the callgrind total of that process,

    ir_warm = Ir(2) - Ir(1)   (one integration after one in-process run)  -- the primary cost
    ir_cold = Ir(1) - Ir(0)   (the first integration of a process)
    deterministic = Ir(1) == Ir(1b)   (exact)

Process start-up, problem construction and output cancel in the differences. `--cachegrind` adds a cachegrind
(`--cache-sim=yes`) run at K = 1 and K = 2 and records every event total and their 2 - 1 difference (Ir, Dr,
Dw, I1mr, D1mr, D1mw, ILmr, DLmr, DLmw); simulated cache counts are a model of a cache, not of this host.

The tool refuses to run unless the binary reports `binary.profile == "bench"` in a `triage-run` row (the first
planned cell is run once, outside valgrind, for this check). A binary that cannot tell release from bench
reports `"release-or-bench"` with `profile_source: "inferred"`; it is accepted only with
`--assert-bench-profile`, an operator assertion that is recorded in every PROFILE row
(`binary.profile_asserted`). Default cells: the CORPUS profile subset
(profile_subset.rtols, dimension <= profile_subset.max_dimension, sweep grids, h0_scale 1.0), narrowed by the
selection filters. The output PROFILE.jsonl is resumable (cells already present are skipped) and written in
plan order; `--force` truncates it. A failed valgrind run gives a row with status `failed`, a reason and null
differences, never 0. No wall clock is recorded.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import importlib.util
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402

CALLGRIND_TAGS = (("0", 0), ("1", 1), ("2", 2), ("1b", 1))
CACHEGRIND_TAGS = (("1", 1), ("2", 2))


def _speed_profile():
    spec = importlib.util.spec_from_file_location("speed_profile_for_triage", tc.TOOLS / "speed_profile.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


event_totals = _speed_profile().event_totals


def binary_profile(binary: str, cell: dict, env: dict) -> tuple[str | None, str]:
    """`binary.profile` of one triage-run row (outside valgrind)."""
    proc = subprocess.run([binary, "triage-run", *tc.cell_args(cell), "--repetitions", "1",
                           "--campaign", "profile-check"], capture_output=True, text=True, env=env)
    if proc.returncode != 0:
        return None, f"triage-run for the profile check: {tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 400)}"
    row = tc.parse_json_output(proc.stdout)
    if row is None:
        return None, "triage-run for the profile check printed no JSON row"
    binary = row.get("binary") or {}
    return binary.get("profile"), f"profile_source {binary.get('profile_source')!r}"


def valgrind_run(valgrind: str, tool: str, out: Path, command: list[str], env: dict, extra: list[str]):
    flag = "--callgrind-out-file" if tool == "callgrind" else "--cachegrind-out-file"
    full = [valgrind, f"--tool={tool}", f"{flag}={out}", *extra, *command]
    return subprocess.run(full, capture_output=True, text=True, env=env)


def profile_cell(args, cell: dict, index: int, env: dict, binary_sha: str, profile: str, prov: dict,
                 valgrind_version: str) -> dict:
    def command(k: int) -> list[str]:
        return [args.binary, "triage-profile-run", *tc.cell_args(cell), "--repetitions", str(k)]

    row = {"schema": tc.PROFILE_SCHEMA, "campaign": args.campaign, "arm": cell["arm"], "problem": cell["problem"],
           "rtol": cell["rtol"], "grid": cell["grid"], "h0_scale": cell["h0_scale"], "status": "ok", "reason": None,
           "ir": {tag: None for tag, _ in CALLGRIND_TAGS}, "ir_warm": None, "ir_cold": None, "deterministic": None,
           "summary": None, "cachegrind": None, "valgrind": valgrind_version,
           "binary": {"sha256": binary_sha, "profile": profile,
                      "profile_asserted": "bench" if args.assert_bench_profile else None},
           "source": {"commit": prov.get("commit"), "tree_clean": prov.get("tree_clean")}}
    stem = f"{index:06d}.{cell['arm']}.{cell['problem']}".replace("/", "_")
    for tag, k in CALLGRIND_TAGS:
        out = args.scratch / f"callgrind.{stem}.{tag}"
        proc = valgrind_run(args.valgrind, "callgrind", out, command(k), env,
                            ["--dump-instr=no", "--compress-strings=yes", "--compress-pos=yes"])
        if proc.returncode != 0:
            row.update(status="failed", reason=f"callgrind K={k} ({tag}): {tc.describe_exit(proc.returncode)}: "
                                               f"{tc.tail(proc.stderr, 800)}")
            return row
        try:
            row["ir"][tag] = int(event_totals(out)["Ir"])
        except (RuntimeError, KeyError, ValueError, OSError) as exc:
            row.update(status="failed", reason=f"callgrind K={k} ({tag}): no Ir total: {exc}")
            return row
        if tag == "1":
            row["summary"] = tc.parse_json_output(proc.stdout)
            if row["summary"] is None:
                row.update(status="failed", reason="K=1 run printed no strict JSON summary")
                return row
        if not args.keep_files:
            out.unlink(missing_ok=True)
    ir = row["ir"]
    row["ir_warm"] = ir["2"] - ir["1"]
    row["ir_cold"] = ir["1"] - ir["0"]
    row["deterministic"] = ir["1"] == ir["1b"]
    if args.cachegrind:
        totals = {}
        for tag, k in CACHEGRIND_TAGS:
            out = args.scratch / f"cachegrind.{stem}.{tag}"
            proc = valgrind_run(args.valgrind, "cachegrind", out, command(k), env, ["--cache-sim=yes"])
            if proc.returncode != 0:
                row["cachegrind"] = {"status": "failed", "reason": f"cachegrind K={k}: "
                                     f"{tc.describe_exit(proc.returncode)}: {tc.tail(proc.stderr, 800)}",
                                     "totals": totals, "warm": None}
                return row
            try:
                totals[tag] = {e: int(v) for e, v in event_totals(out).items()}
            except (RuntimeError, ValueError, OSError) as exc:
                row["cachegrind"] = {"status": "failed", "reason": f"cachegrind K={k}: {exc}", "totals": totals,
                                     "warm": None}
                return row
            if not args.keep_files:
                out.unlink(missing_ok=True)
        events = sorted(set(totals["1"]) & set(totals["2"]))
        row["cachegrind"] = {"status": "ok", "reason": None, "totals": totals,
                             "warm": {e: totals["2"][e] - totals["1"][e] for e in events}}
    return row


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, help="rodas5p CLI binary built with the bench profile")
    parser.add_argument("--list", type=Path, help="triage-list JSON (default: run `BINARY triage-list`)")
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="PROFILE.jsonl (appended; resumable)")
    parser.add_argument("--campaign", required=True)
    parser.add_argument("--scratch", required=True, type=Path, help="directory for valgrind output files")
    parser.add_argument("--valgrind", default="valgrind", help="valgrind executable (default: valgrind)")
    parser.add_argument("--cachegrind", action="store_true", help="also record cachegrind event totals (2 - 1)")
    parser.add_argument("--keep-files", action="store_true", help="keep the valgrind output files")
    parser.add_argument("--all-cells", action="store_true",
                        help="plan the full run sweep instead of the CORPUS profile subset")
    parser.add_argument("-j", "--jobs", type=int, default=1)
    parser.add_argument("--force", action="store_true", help="truncate an existing output instead of resuming")
    parser.add_argument("--assert-bench-profile", action="store_true",
                        help="accept a binary reporting 'release-or-bench' (inferred) as built with the bench "
                             "profile; the assertion is recorded in every row")
    tc.add_selection_arguments(parser)
    args = parser.parse_args(argv)
    try:
        list_doc, _ = tc.list_document(args.list, args.binary)
        corpus = tc.load_json(args.corpus)
        cells, notes = tc.plan_cells(list_doc, corpus, "runs" if args.all_cells else "profile", **tc.selection(args))
    except tc.TriageError as exc:
        for reason in exc.reasons:
            print("error:", reason, file=sys.stderr)
        return 2
    for note in notes:
        print("note:", note, file=sys.stderr)
    if not cells:
        print("no cells planned", file=sys.stderr)
        return 0
    prov = tc.git_provenance()
    env = tc.provenance_env(prov)
    profile, detail = binary_profile(args.binary, cells[0], env)
    if not (profile == "bench" or (profile == "release-or-bench" and args.assert_bench_profile)):
        hint = " (pass --assert-bench-profile if it was built with --profile bench)" \
            if profile == "release-or-bench" else ""
        print(f"refused: binary profile is {profile!r}, not 'bench' ({detail}){hint}", file=sys.stderr)
        return 2
    try:
        version = subprocess.run([args.valgrind, "--version"], capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        print(f"refused: valgrind unavailable: {exc}", file=sys.stderr)
        return 2
    if args.force and args.output.exists():
        args.output.unlink()
    done = {tc.cell_key(r) for r in tc.load_jsonl(args.output)} if args.output.exists() else set()
    todo = [c for c in cells if (args.campaign, c["arm"], c["problem"], c["rtol"], c["grid"], c["h0_scale"])
            not in done]
    print(f"{len(cells)} planned cells, {len(todo)} to profile", file=sys.stderr)
    args.scratch.mkdir(parents=True, exist_ok=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    binary_sha = tc.sha256_file(Path(args.binary))
    appender = tc.OrderedAppender(args.output, len(todo))
    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
            futures = {pool.submit(profile_cell, args, c, i, env, binary_sha, profile, prov, version): i
                       for i, c in enumerate(todo)}
            for future in concurrent.futures.as_completed(futures):
                index = futures[future]
                row = future.result()
                print(f"{row['arm']} {row['problem']} rtol={row['rtol']:g} {row['grid']}: {row['status']} "
                      f"ir_warm={row['ir_warm']} deterministic={row['deterministic']}", file=sys.stderr, flush=True)
                appender.add(index, row)
    finally:
        appender.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
