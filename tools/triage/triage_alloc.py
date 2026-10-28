#!/usr/bin/env python3
"""Allocation counts of component-triage cells (`rodas5p-allocation-audit triage`, CT-HARNESS v1 section 2).

One subprocess per cell:

    rodas5p-allocation-audit triage --arm A --problem P --rtol R --grid G --h0-scale S --warmups K --windows M

The binary runs one `cold` integration, K untimed integrations and M integrations each inside its own counting
window, and prints one vigilode-triage-alloc-v1 document. This tool stores the document's raw `cold` and
`windows` unchanged and derives

    warm.<field> = max over the M windows of the raw window value      (no subtraction, no saturation)

for allocations, deallocations, allocated_bytes and peak_live_bytes. A cell whose process aborts, prints no
document or prints a document for another cell gets a row with status `process_abort` / `invalid`, its exit
status and stderr tail, and null counts (never 0).

Default cells: the CORPUS profile subset (as tools/triage/triage_profile.py). ALLOCATIONS.jsonl is resumable
(present cells are skipped) and written in plan order; `--force` truncates it.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import triage_common as tc  # noqa: E402

COUNT_FIELDS = ("allocations", "deallocations", "allocated_bytes", "peak_live_bytes")


def window_problems(doc: dict, cell: dict, warmups: int, windows: int) -> list[str]:
    reasons = []
    if doc.get("schema") != tc.ALLOC_SCHEMA:
        reasons.append(f"schema {doc.get('schema')!r} != {tc.ALLOC_SCHEMA!r}")
    for key in ("arm", "problem", "grid"):
        if doc.get(key) != cell[key]:
            reasons.append(f"{key} {doc.get(key)!r} != requested {cell[key]!r}")
    for key in ("rtol", "h0_scale"):
        if not tc.finite(doc.get(key)) or float(doc[key]) != float(cell[key]):
            reasons.append(f"{key} {doc.get(key)!r} != requested {cell[key]!r}")
    if doc.get("warmups") != warmups:
        reasons.append(f"warmups {doc.get('warmups')!r} != requested {warmups}")
    records = [("cold", doc.get("cold"))] + [(f"windows[{i}]", w) for i, w in enumerate(doc.get("windows") or [])]
    if not isinstance(doc.get("windows"), list) or len(doc["windows"]) != windows:
        reasons.append(f"expected {windows} windows, got {doc.get('windows')!r:.80}")
    for name, rec in records:
        if not isinstance(rec, dict) or set(rec) != set(COUNT_FIELDS):
            reasons.append(f"{name}: key set is not {sorted(COUNT_FIELDS)}")
            continue
        for field in COUNT_FIELDS:
            value = rec[field]
            if type(value) is not int or (field != "peak_live_bytes" and value < 0):
                reasons.append(f"{name}.{field}: expected {'int' if field == 'peak_live_bytes' else 'int >= 0'}, "
                               f"got {value!r}")
    return reasons


def alloc_cell(args, cell: dict, env: dict, binary_sha: str, prov: dict) -> dict:
    command = [args.binary, "triage", *tc.cell_args(cell), "--warmups", str(args.warmups),
               "--windows", str(args.windows)]
    row = {"schema": tc.ALLOCATIONS_SCHEMA, "campaign": args.campaign, "arm": cell["arm"], "problem": cell["problem"],
           "rtol": cell["rtol"], "grid": cell["grid"], "h0_scale": cell["h0_scale"], "warmups": args.warmups,
           "windows_requested": args.windows, "status": "ok", "reason": None, "cold": None, "windows": None,
           "warm": None, "warm_rule": "max over windows of the raw window counts", "process": None,
           "binary": {"sha256": binary_sha},
           "source": {"commit": prov.get("commit"), "tree_clean": prov.get("tree_clean")}}
    try:
        proc = subprocess.run(command, capture_output=True, text=True, env=env, timeout=args.timeout)
    except subprocess.TimeoutExpired as exc:
        row.update(status="process_abort", reason=f"timeout after {args.timeout} s",
                   process={"exit_status": None, "stderr_tail": tc.tail(exc.stderr)})
        return row
    if proc.returncode != 0:
        row.update(status="process_abort", reason=tc.describe_exit(proc.returncode),
                   process={"exit_status": proc.returncode, "stderr_tail": tc.tail(proc.stderr)})
        return row
    doc = tc.parse_json_output(proc.stdout)
    if doc is None:
        row.update(status="invalid", reason="exit 0 without a strict JSON document",
                   process={"exit_status": 0, "stderr_tail": tc.tail(proc.stderr)})
        return row
    problems = window_problems(doc, cell, args.warmups, args.windows)
    if problems:
        row.update(status="invalid", reason="; ".join(problems[:10]),
                   process={"exit_status": 0, "stderr_tail": tc.tail(proc.stderr)})
        return row
    row["cold"] = doc["cold"]
    row["windows"] = doc["windows"]
    row["warm"] = {f: max(w[f] for w in doc["windows"]) for f in COUNT_FIELDS} if doc["windows"] else None
    if row["warm"] is None:
        row.update(status="invalid", reason="no windows: warm counts undefined")
    return row


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True, help="rodas5p-allocation-audit binary")
    parser.add_argument("--list", required=True, type=Path, help="triage-list JSON of the rodas5p binary")
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path, help="ALLOCATIONS.jsonl (appended; resumable)")
    parser.add_argument("--campaign", required=True)
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--windows", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=None)
    parser.add_argument("--all-cells", action="store_true", help="plan the full run sweep instead of the subset")
    parser.add_argument("-j", "--jobs", type=int, default=1)
    parser.add_argument("--force", action="store_true", help="truncate an existing output instead of resuming")
    tc.add_selection_arguments(parser)
    args = parser.parse_args(argv)
    if args.warmups < 0 or args.windows < 1:
        parser.error("--warmups must be >= 0 and --windows >= 1")
    try:
        list_doc, _ = tc.list_document(args.list, None)
        corpus = tc.load_json(args.corpus)
        cells, notes = tc.plan_cells(list_doc, corpus, "runs" if args.all_cells else "alloc", **tc.selection(args))
    except tc.TriageError as exc:
        for reason in exc.reasons:
            print("error:", reason, file=sys.stderr)
        return 2
    for note in notes:
        print("note:", note, file=sys.stderr)
    if args.force and args.output.exists():
        args.output.unlink()
    done = {tc.cell_key(r) for r in tc.load_jsonl(args.output)} if args.output.exists() else set()
    todo = [c for c in cells if (args.campaign, c["arm"], c["problem"], c["rtol"], c["grid"], c["h0_scale"])
            not in done]
    print(f"{len(cells)} planned cells, {len(todo)} to measure", file=sys.stderr)
    prov = tc.git_provenance()
    env = tc.provenance_env(prov)
    binary_sha = tc.sha256_file(Path(args.binary)) if Path(args.binary).is_file() else None
    args.output.parent.mkdir(parents=True, exist_ok=True)
    appender = tc.OrderedAppender(args.output, len(todo))
    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
            futures = {pool.submit(alloc_cell, args, c, env, binary_sha, prov): i for i, c in enumerate(todo)}
            for future in concurrent.futures.as_completed(futures):
                row = future.result()
                print(f"{row['arm']} {row['problem']} rtol={row['rtol']:g} {row['grid']}: {row['status']} "
                      f"warm={row['warm']}", file=sys.stderr, flush=True)
                appender.add(futures[future], row)
    finally:
        appender.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
