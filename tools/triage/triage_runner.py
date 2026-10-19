#!/usr/bin/env python3
"""Run component-triage cells through `rodas5p triage-run`, one subprocess per cell (CT-HARNESS v1, section 1).

Cells are the wired internal arms of `rodas5p triage-list` x the applicable CORPUS problems x the CORPUS sweep
(rtols x grids x h0_scales), narrowed by the selection filters. Every planned cell produces exactly one row in
RUNS.jsonl:

* the binary's own row (schema vigilode-triage-row-v1) when it exits 0 with a row for exactly the requested cell;
* otherwise a typed `process_abort` row in the same schema, carrying `process` = {kind, exit_status, signal,
  stderr_tail, stdout_tail}. Kinds: `abort` (nonzero exit or signal; release profile is panic=abort),
  `usage_exit_2` (unknown arm/problem or inapplicable pair), `unparseable_row`, `cell_mismatch`, `timeout`.
  A failed cell is never dropped.

The driver sets VIGILODE_SOURCE_COMMIT / VIGILODE_TREE_CLEAN from git (the binary does not shell out to git).
The output is resumable: cells already present (same campaign, arm, problem, rtol, grid, h0_scale) are skipped,
including process_abort cells. Rows are appended in plan order even with -j > 1, so a rerun of an identical
plan writes identical bytes. `--force` truncates the output instead of resuming.

    python3 tools/triage/triage_runner.py --binary target/release/rodas5p \
        --corpus research/component_triage_20261019/CORPUS.json --campaign ct-20261019-smoke \
        --output research/component_triage_20261019/RUNS.jsonl -j 4 --tier G
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402


parse_row = tc.parse_json_output


def mismatch(row: dict, cell: dict, campaign: str) -> list[str]:
    reasons = []
    if row.get("schema") != tc.ROW_SCHEMA:
        reasons.append(f"schema {row.get('schema')!r} != {tc.ROW_SCHEMA!r}")
    for key in ("arm", "problem", "grid"):
        if row.get(key) != cell[key]:
            reasons.append(f"{key} {row.get(key)!r} != requested {cell[key]!r}")
    for key in ("rtol", "h0_scale"):
        if not tc.finite(row.get(key)) or float(row[key]) != float(cell[key]):
            reasons.append(f"{key} {row.get(key)!r} != requested {cell[key]!r}")
    if row.get("campaign") != campaign:
        reasons.append(f"campaign {row.get('campaign')!r} != requested {campaign!r}")
    if row.get("status") not in tc.ROW_STATUSES - {"process_abort", "unavailable"}:
        reasons.append(f"status {row.get('status')!r} is not a binary row status")
    missing = sorted(tc.ROW_KEYS - set(row))
    if missing:
        reasons.append(f"missing row keys {missing}")
    return reasons


def abort_row(cell: dict, campaign: str, repetitions: int, components: list[str], problem: dict,
              binary_sha: str | None, prov: dict, kind: str, code: int | None, stderr: str, stdout: str,
              message: str) -> dict:
    signal_name = None
    if code is not None and code < 0:
        signal_name = tc.describe_exit(code)
    atol = cell["rtol"] * float(problem["atol_scale"]) if tc.finite(problem.get("atol_scale")) else None
    try:
        h0 = cell["h0_scale"] * tc.h0_base(problem)
    except (KeyError, TypeError, ValueError):
        h0 = None
    return {
        "schema": tc.ROW_SCHEMA, "campaign": campaign, "arm": cell["arm"], "components": list(components),
        "problem": cell["problem"], "tier": cell.get("tier"), "dimension": cell.get("dimension"),
        "rtol": cell["rtol"], "atol": atol, "grid": cell["grid"], "h0_scale": cell["h0_scale"], "h0": h0,
        "status": "process_abort", "error": message, "success": False,
        "attempts": None, "accepted_steps": None, "rejected_steps": None,
        "output_times": [], "states_hex": [], "counters": None, "extra_work": None, "counter_semantics": None,
        "deterministic": None, "repetitions": repetitions,
        "binary": {"sha256": binary_sha, "profile": None},
        "source": {"commit": prov.get("commit"), "tree_clean": prov.get("tree_clean")},
        "process": {"kind": kind, "exit_status": code, "signal": signal_name,
                    "stderr_tail": tc.tail(stderr), "stdout_tail": tc.tail(stdout)},
    }


def run_cell(binary: str, cell: dict, campaign: str, repetitions: int, env: dict, timeout: float | None,
             components: list[str], problem: dict, binary_sha: str | None, prov: dict) -> dict:
    command = [binary, "triage-run", *tc.cell_args(cell), "--repetitions", str(repetitions), "--campaign", campaign]
    common = (cell, campaign, repetitions, components, problem, binary_sha, prov)
    try:
        proc = subprocess.run(command, capture_output=True, text=True, env=env, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        return abort_row(*common, "timeout", None, tc.tail(exc.stderr), tc.tail(exc.stdout),
                         f"timeout after {timeout} s")
    if proc.returncode == 2:
        return abort_row(*common, "usage_exit_2", 2, proc.stderr, proc.stdout,
                         "exit status 2 (unknown arm/problem or inapplicable pair)")
    if proc.returncode != 0:
        return abort_row(*common, "abort", proc.returncode, proc.stderr, proc.stdout, tc.describe_exit(proc.returncode))
    row = parse_row(proc.stdout)
    if row is None:
        return abort_row(*common, "unparseable_row", 0, proc.stderr, proc.stdout, "exit 0 without a JSON row")
    reasons = mismatch(row, cell, campaign)
    if reasons:
        return abort_row(*common, "cell_mismatch", 0, proc.stderr, proc.stdout,
                         "row does not match the requested cell: " + "; ".join(reasons))
    return row


def existing_keys(path: Path) -> set:
    if not path.exists():
        return set()
    return {tc.cell_key(r) for r in tc.load_jsonl(path)}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, help="rodas5p CLI binary")
    parser.add_argument("--list", type=Path, help="triage-list JSON (default: run `BINARY triage-list`)")
    parser.add_argument("--corpus", required=True, type=Path, help="CORPUS.json")
    parser.add_argument("--output", required=True, type=Path, help="RUNS.jsonl (appended; resumable)")
    parser.add_argument("--campaign", required=True, help="campaign id passed to the binary and keyed on resume")
    parser.add_argument("--repetitions", type=int, default=1, help="in-process repetitions K per cell (default 1)")
    parser.add_argument("-j", "--jobs", type=int, default=1, help="parallel subprocesses (default 1)")
    parser.add_argument("--timeout", type=float, default=None, help="seconds per cell (default: none)")
    parser.add_argument("--force", action="store_true", help="truncate an existing output instead of resuming")
    parser.add_argument("--plan-only", action="store_true", help="print the cells that would run and exit")
    tc.add_selection_arguments(parser)
    args = parser.parse_args(argv)
    if args.repetitions < 1 or args.jobs < 1:
        parser.error("--repetitions and --jobs must be >= 1")
    try:
        list_doc, _ = tc.list_document(args.list, args.binary)
        corpus = tc.load_json(args.corpus)
        cells, notes = tc.plan_cells(list_doc, corpus, "runs", **tc.selection(args))
    except tc.TriageError as exc:
        for reason in exc.reasons:
            print("error:", reason, file=sys.stderr)
        return 2
    for note in notes:
        print("note:", note, file=sys.stderr)
    if args.force and args.output.exists():
        args.output.unlink()
    done = existing_keys(args.output)
    todo = [c for c in cells if (args.campaign, c["arm"], c["problem"], c["rtol"], c["grid"], c["h0_scale"])
            not in done]
    print(f"{len(cells)} planned cells, {len(cells) - len(todo)} already present, {len(todo)} to run",
          file=sys.stderr)
    if args.plan_only:
        for c in todo:
            print(json.dumps(c, sort_keys=True))
        return 0
    prov = tc.git_provenance()
    env = tc.provenance_env(prov)
    binary_sha = tc.sha256_file(Path(args.binary)) if Path(args.binary).is_file() else None
    arms = {a["id"]: a for a in list_doc.get("arms", [])}
    list_problems = {q["id"]: q for q in list_doc.get("problems", [])}
    problems = {p["id"]: tc.merged_problem(p, list_problems.get(p["id"])) for p in corpus.get("problems", [])}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    appender = tc.OrderedAppender(args.output, len(todo))
    aborts = 0
    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = {pool.submit(run_cell, args.binary, c, args.campaign, args.repetitions, env, args.timeout,
                                   arms[c["arm"]].get("components", []), problems[c["problem"]], binary_sha,
                                   prov): i for i, c in enumerate(todo)}
            for future in concurrent.futures.as_completed(futures):
                index = futures[future]
                row = future.result()
                if row["status"] == "process_abort":
                    aborts += 1
                cell = todo[index]
                print(f"{cell['arm']} {cell['problem']} rtol={cell['rtol']:g} {cell['grid']} "
                      f"h0x{cell['h0_scale']:g}: {row['status']}", file=sys.stderr, flush=True)
                appender.add(index, row)
    finally:
        appender.close()
    print(f"wrote {len(todo)} rows to {args.output} ({aborts} process_abort)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
