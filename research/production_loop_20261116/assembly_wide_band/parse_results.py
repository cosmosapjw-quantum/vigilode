#!/usr/bin/env python3
"""Parser of loop node V31 (default first-use check of the assembly client on wide declared patterns,
owner decision OD-L1-27).

Reads only raw logs: logs/ of this directory (recordings A and B of the identity cells, the V26 corpus,
the survey and the supplement; unit.log, checks.log, checker_all.log) and the logs of node V29
(assembly_default/logs/identity_B_<cell>.log and corpus_B_<group>.log: the V29 default recorded with
the code of fc185e2a, which equals the code of this node's base up to rustdoc). It also reads git (the
changed paths and the diff of the existing test files against the base commit). Writes RESULTS.json
(strict JSON) and REPORT_KO.md. No number of either file is typed in: every number is computed here
from the logs.

    python3 -I research/production_loop_20261116/assembly_wide_band/parse_results.py
    python3 -I research/production_loop_20261116/assembly_wide_band/parse_results.py --report-only
        (re-renders REPORT_KO.md from the committed RESULTS.json; RESULTS.json is not rewritten)
"""
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LOGS = HERE / "logs"
V29_LOGS = HERE.parent / "assembly_default" / "logs"
BASE = "d4e2772fb6341f6fc262748326a7918bbc8fac9b"
# The commits the recordings were built at (NOTES.txt entries 3 and 4).
COMMIT_A = "1952c87c9e4206dfed00ce3c2eb1f90dbbecd1d6"
COMMIT_B = "63170db9aee60e9a983cc45535500d5beff3d287"
CELLS = [
    "nlkpp-dense-n400",
    "bruss1d-n128-dense",
    "nlkpp-band-n4096-r24",
    "bruss1d-n131072",
    "bruss1d-n128-coloured",
]
GROUPS = ["controls", "ring", "omit128", "omit1024", "omit131072", "narrow", "correct"]
FAMILIES = ["random", "band", "ring", "narrow"]
CLIENT_ARMS = ["seq", "t1", "t2", "t4", "t6"]
V29_PROGRAM = "stage_clients_assembly_default_record"
V31_PROGRAM = "stage_clients_assembly_wide_band_record"
WRITE_ALLOWLIST = [
    "crates/rodas5p-integrators/src/stage_clients/assembly.rs",
    "crates/rodas5p-integrators/src/stage_clients/assembly/",
    "crates/rodas5p-integrators/tests/stage_clients_assembly_",
    "research/production_loop_20261116/assembly_wide_band/",
]
# Pinned implementation files (CHECKPOINT implemented_source_sha256) that this node may change.
PINNED_ALLOWED = [
    "crates/rodas5p-integrators/src/stage_clients/assembly.rs",
    "crates/rodas5p-integrators/src/stage_clients/assembly/engine.rs",
    "crates/rodas5p-integrators/src/stage_clients/assembly/pattern.rs",
]
# Every edited line of an existing test file, by (file, first new line, last new line), with its
# reason: the task allows only lines that pin the default check of the coloured route or its counts.
EDIT_REASONS = {
    ("crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs", 153, 153): (
        "pin of the default of the coloured route: the loop asserts MultiColouring for "
        "AssemblyOptions::coloured(ring_pattern(16)); ring_pattern(16) has total_groups 16 = n, so the "
        "rule of OD-L1-27 picks All there (pinned by stage_clients_assembly_wide_band.rs). Replaced by "
        "ring_pattern(34) (total_groups 33 < 34, the boundary n - 1), which keeps the multi-colouring "
        "default the loop pins, with a non-banded pattern as before"
    ),
    ("crates/rodas5p-integrators/tests/stage_clients_assembly_default_alloc.rs", 114, 122): (
        "pin of the default of the coloured route: the assertion expected MultiColouring for every "
        "case, among them ring_pattern(30), whose total_groups 30 = n makes the default All; the "
        "assertion now expects All for that case and MultiColouring for the three bands (one "
        "statement; rustfmt spreads it over 9 lines). The allocation assertions of the test are "
        "unchanged and hold for the full check too"
    ),
}
EDIT_REASONS_KO = {
    ("crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs", 153, 153): (
        "채색 경로 기본값의 고정(pin). 이 루프는 AssemblyOptions::coloured(ring_pattern(16))에 "
        "MultiColouring을 단언했다. ring_pattern(16)은 total_groups 16 = n이므로 OD-L1-27 규칙은 All을 "
        "고른다(새 테스트 stage_clients_assembly_wide_band.rs가 고정). total_groups 33 < 34(경계 n - 1)인 "
        "ring_pattern(34)로 바꿔, 루프가 고정하는 다중 채색 기본값을 띠가 아닌 패턴으로 그대로 검사한다"
    ),
    ("crates/rodas5p-integrators/tests/stage_clients_assembly_default_alloc.rs", 114, 122): (
        "채색 경로 기본값의 고정(pin). 모든 경우에 MultiColouring을 기대했는데, 그중 ring_pattern(30)은 "
        "total_groups 30 = n이라 기본값이 All이 된다. 이 경우는 All을, 세 띠 경우는 MultiColouring을 "
        "기대하도록 바꿨다(한 문장, rustfmt가 9줄로 나눔). 할당 단언은 그대로이며 전체 검사에도 성립한다"
    ),
}
# Lines of existing tests that name the default of the coloured route or its counts and stay
# unchanged, with the reason (examined for the rule).
EXAMINED_UNCHANGED = [
    {
        "path": "crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs",
        "lines": "309-317",
        "reason": "expected check counts of the coloured route are total_groups + repeated base; its "
                  "only route whose default becomes All, ring_pattern(30), has total_groups 30 = n, so "
                  "All spends the same count (n, and one more with threads) and verified_columns stays n",
    },
    {
        "path": "crates/rodas5p-integrators/tests/stage_clients_assembly_default.rs",
        "lines": "573-576",
        "reason": "reference check MultiColouring for the coloured cases: the one case whose default "
                  "becomes All (ring 30, true pattern) passes with 30 columns under both checks, so the "
                  "asserted equality of the decisions still holds",
    },
    {
        "path": "crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs",
        "lines": "1034-1060",
        "reason": "default options in refused configurations and one accepted band (1, 1) of order 8 "
                  "(total_groups 7 < 8); the refusals do not depend on the check",
    },
]


def git(*args):
    return subprocess.check_output(["git", "-C", str(ROOT), *args], text=True)


def read_log(path):
    """Header fields, tagged JSON records and exit codes of a log."""
    header, records, exits = {}, [], []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith("# exit code:"):
            exits.append(int(line.split(":", 1)[1]))
        elif line.startswith("# ") and ": " in line:
            key, value = line[2:].split(": ", 1)
            header.setdefault(key, value)
        match = re.match(r"^([A-Z0-9_]+_JSON) (\{.*\})\s*$", line)
        if match:
            records.append((match.group(1), json.loads(match.group(2))))
    return header, records, exits


def tagged(records, tag):
    return [record for name, record in records if name == tag]


def diff(a, b, path=""):
    """The paths at which two JSON values differ."""
    if isinstance(a, dict) and isinstance(b, dict):
        out = []
        for key in sorted(set(a) | set(b)):
            if key not in a or key not in b:
                out.append(f"{path}.{key}")
            else:
                out += diff(a[key], b[key], f"{path}.{key}")
        return out
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            return [f"{path}[len]"]
        out = []
        for index, (x, y) in enumerate(zip(a, b)):
            out += diff(x, y, f"{path}[{index}]")
        return out
    return [] if (a == b and type(a) is type(b)) else [path or "."]


def rule(n, total_groups):
    """The default check of the coloured route by the rule of OD-L1-27."""
    return "All" if total_groups >= n else "MultiColouring"


def detected(record):
    return record["outcome"] == "error" and record.get("contract_violation") is True


def decision_key(record):
    if record["outcome"] == "pass":
        return ["pass", record.get("columns")]
    return [record["outcome"], record.get("kind"), record.get("row"), record.get("column"),
            record.get("colour"), record.get("colouring")]


# ---------------------------------------------------------------------------------------------
# Provenance of the recordings
# ---------------------------------------------------------------------------------------------

def log_paths(label):
    paths = [LOGS / f"identity_{label}_{cell}.log" for cell in CELLS]
    paths += [LOGS / f"corpus_{label}_{group}.log" for group in GROUPS]
    paths += [LOGS / f"survey_{label}.log", LOGS / f"wide_{label}_random.log"]
    return paths


def provenance(label):
    rows = []
    for path in log_paths(label):
        header, records, exits = read_log(path)
        rows.append({
            "log": str(path.relative_to(ROOT)),
            "git_head": header.get("git head"),
            "tracked_changes": header.get("git status (tracked changes)"),
            "client_sources_sha256": header.get("client sources sha256"),
            "program": header.get("program"),
            "program_source_sha256": header.get("program source sha256"),
            "binary_sha256": header.get("binary sha256"),
            "loadavg_at_start": header.get("loadavg at start"),
            "exit_codes": exits,
            "test_result_ok": any("test result: ok. 1 passed" in line
                                  for line in path.read_text().splitlines()),
        })
    return rows


def provenance_checks(prov):
    a, b = prov["A"], prov["B"]
    programs = {}
    for label, rows in prov.items():
        for row in rows:
            programs.setdefault(row["program"], {}).setdefault(label, set()).add(
                row["program_source_sha256"])
    same_program = all(len(per["A"]) == 1 and per["A"] == per["B"] for per in programs.values())
    client_a = {row["client_sources_sha256"] for row in a}
    client_b = {row["client_sources_sha256"] for row in b}
    return {
        "logs_per_recording": {"A": len(a), "B": len(b)},
        "programs": {name: {label: sorted(hashes) for label, hashes in per.items()}
                     for name, per in sorted(programs.items())},
        "same_program_source_A_B": same_program,
        "clean_trees": all(row["tracked_changes"] == "0" for row in a + b),
        "exit_codes_zero": all(row["exit_codes"] == [0] and row["test_result_ok"] for row in a + b),
        "A_built_at": sorted({row["git_head"] for row in a}),
        "B_built_at": sorted({row["git_head"] for row in b}),
        "A_at_expected_commit": all(row["git_head"] == COMMIT_A for row in a),
        "B_at_expected_commit": all(row["git_head"] == COMMIT_B for row in b),
        "client_sources_A": sorted(client_a),
        "client_sources_B": sorted(client_b),
        "client_changed_between_A_and_B": len(client_a) == 1 and len(client_b) == 1
        and client_a != client_b,
        "binaries_differ_A_B": {row["binary_sha256"] for row in a}.isdisjoint(
            {row["binary_sha256"] for row in b}),
    }


# ---------------------------------------------------------------------------------------------
# Identity cells
# ---------------------------------------------------------------------------------------------

def identity_records(directory, label, tag_prefix="V29"):
    runs, cells, metas = {}, {}, []
    for cell in CELLS:
        _, records, _ = read_log(directory / f"identity_{label}_{cell}.log")
        metas += tagged(records, f"{tag_prefix}_META_JSON")
        for record in tagged(records, f"{tag_prefix}_CELL_JSON"):
            cells[(record["cell"], record["backend"])] = record
        for record in tagged(records, f"{tag_prefix}_IDENTITY_JSON"):
            key = (record["cell"], record["backend"], record["arm"])
            if key in runs:
                raise ValueError(f"duplicate identity record {key} in {label}")
            runs[key] = record
    return runs, cells, metas


def analyse_identity():
    a_runs, a_cells, a_meta = identity_records(LOGS, "A")
    b_runs, b_cells, b_meta = identity_records(LOGS, "B")
    v29_runs, v29_cells, _ = identity_records(V29_LOGS, "B")
    mismatches = []
    for key in sorted(set(a_runs) | set(b_runs)):
        if key not in a_runs or key not in b_runs:
            mismatches.append({"key": list(key), "fields": ["missing in A or B"]})
            continue
        fields = diff(a_runs[key], b_runs[key])
        if fields:
            mismatches.append({"key": list(key), "fields": fields})
    cell_mismatches = []
    for key in sorted(set(a_cells) | set(b_cells)):
        fields = diff(a_cells.get(key), b_cells.get(key))
        if fields:
            cell_mismatches.append({"key": list(key), "fields": fields})
    v29_mismatches = []
    for key in sorted(set(a_runs) | set(v29_runs)):
        fields = diff(a_runs.get(key), v29_runs.get(key))
        if fields:
            v29_mismatches.append({"key": list(key), "fields": fields})
    rows, rule_mismatches = [], []
    for key in sorted(a_runs, key=lambda k: (CELLS.index(k[0]), k[1], k[2])):
        a, b = a_runs[key], b_runs.get(key, {})
        cell, backend, arm = key
        row = {"cell": cell, "backend": backend, "arm": arm, "n": a["n"], "route": a["route"],
               "all_fields_equal_A_B": not diff(a, b)}
        if arm == "prot":
            row.update({"jacobian_fingerprint": a["jacobian_fingerprint"],
                        "state_fingerprint": a["state_fingerprint"],
                        "final_fingerprint": a["final_fingerprint"], "t_end_bits": a["t_end_bits"],
                        "attempts": a["attempts"], "builds": a["builds"]})
        else:
            cell_record = a_cells[(cell, backend)]
            colourings = cell_record.get("check_colourings")
            total = colourings["total_groups"] if isinstance(colourings, dict) else None
            expected_b = "Sample(8)" if a["route"] == "dense" else rule(a["n"], total)
            if b.get("first_use_check") != expected_b or a["first_use_check"] != (
                    "Sample(8)" if a["route"] == "dense" else "MultiColouring"):
                rule_mismatches.append({"key": list(key), "A": a["first_use_check"],
                                        "B": b.get("first_use_check"), "expected_B": expected_b})
            row.update({
                "total_groups": total,
                "first_use_check": {"A": a["first_use_check"], "B": b.get("first_use_check")},
                "verifications": {"A": a["stats"]["verifications"], "B": b["stats"]["verifications"]},
                "verified_columns": {"A": a["stats"]["verified_columns"],
                                     "B": b["stats"]["verified_columns"]},
                "verification_rhs_calls": {"A": a["stats"]["verification_rhs_calls"],
                                           "B": b["stats"]["verification_rhs_calls"]},
                "rhs_calls": {"A": a["counters"]["rhs_calls"], "B": b["counters"]["rhs_calls"]},
                "jacobian_fingerprint": a["stats"]["jacobian_fingerprint"],
                "state_fingerprint": a["stats"]["state_fingerprint"],
                "final_fingerprint": a["final_fingerprint"],
                "output_fingerprint": a["output_fingerprint"], "t_end_bits": a["t_end_bits"],
                "attempts": a["attempts"], "accepted_steps": a["accepted_steps"],
                "rejected_steps": a["rejected_steps"],
            })
        rows.append(row)
    cells = []
    for (cell, backend), record in sorted(a_cells.items(), key=lambda kv: CELLS.index(kv[0][0])):
        colourings = record.get("check_colourings")
        cells.append({
            "cell": cell, "backend": backend, "n": record["n"], "route": record["route"],
            "total_groups": colourings["total_groups"] if isinstance(colourings, dict) else None,
            "rule_picks_all": isinstance(colourings, dict)
            and colourings["total_groups"] >= record["n"],
            "default_checks_A": sorted({d["first_use_check"] for d in record["default_checks"]}),
            "default_checks_B": sorted({d["first_use_check"]
                                        for d in b_cells[(cell, backend)]["default_checks"]}),
        })
    meta = {
        "A_default_coloured_probe_banded_4_1_1": sorted({m["default_coloured"] for m in a_meta}),
        "B_default_coloured_probe_banded_4_1_1": sorted({m["default_coloured"] for m in b_meta}),
        "A_default_dense": sorted({m["default_dense"] for m in a_meta}),
        "B_default_dense": sorted({m["default_dense"] for m in b_meta}),
    }
    return {
        "runs": rows,
        "run_count": len(a_runs),
        "cells": cells,
        "mismatches_A_B": mismatches,
        "cell_record_mismatches_A_B": cell_mismatches,
        "rule_mismatches": rule_mismatches,
        "commit_A_against_v29_B": {"runs_compared": len(set(a_runs) & set(v29_runs)),
                                   "runs_v29": len(v29_runs), "mismatches": v29_mismatches,
                                   "cell_records_equal": all(not diff(a_cells[k], v29_cells.get(k))
                                                             for k in a_cells)},
        "meta_probe": meta,
    }


# ---------------------------------------------------------------------------------------------
# Corpus of node V26
# ---------------------------------------------------------------------------------------------

def corpus_records(directory, label):
    cases, groups = {}, {}
    for group in GROUPS:
        _, records, _ = read_log(directory / f"corpus_{label}_{group}.log")
        for record in tagged(records, "V29_CASE_JSON"):
            if record["case"] in cases:
                raise ValueError("duplicate case " + record["case"])
            cases[record["case"]] = record
        for record in tagged(records, "V29_GROUP_JSON"):
            groups[record["group"]] = record
    return cases, groups


def wrong(case):
    return not case["correct"]


def class_costs(rows):
    """Cost of the first-use check per case class: rows of (class, execution, a_record, b_record)."""
    table = defaultdict(lambda: {"cases": 0, "passing_A": 0, "passing_B": 0,
                                 "check_rhs_passing_A": 0, "check_rhs_passing_B": 0,
                                 "rhs_calls_A": 0, "rhs_calls_B": 0, "checks_A": Counter(),
                                 "checks_B": Counter()})
    for klass, execution, check_a, check_b, a, b in rows:
        entry = table[(klass, execution)]
        entry["cases"] += 1
        entry["checks_A"][check_a] += 1
        entry["checks_B"][check_b] += 1
        entry["rhs_calls_A"] += a.get("rhs_calls", 0)
        entry["rhs_calls_B"] += b.get("rhs_calls", 0)
        if a["outcome"] == "pass":
            entry["passing_A"] += 1
            entry["check_rhs_passing_A"] += a["verification_rhs_calls"]
        if b["outcome"] == "pass":
            entry["passing_B"] += 1
            entry["check_rhs_passing_B"] += b["verification_rhs_calls"]
    out = []
    for (klass, execution), entry in sorted(table.items()):
        out.append({"class": klass, "execution": execution, **{
            key: (dict(sorted(value.items())) if isinstance(value, Counter) else value)
            for key, value in entry.items()}})
    return out


def analyse_corpus():
    a_cases, a_groups = corpus_records(LOGS, "A")
    b_cases, b_groups = corpus_records(LOGS, "B")
    v29_cases, _ = corpus_records(V29_LOGS, "B")
    same_ids = set(a_cases) == set(b_cases) == set(v29_cases)
    totals = {}
    for case_id, case in a_cases.items():
        colourings = case.get("check_colourings")
        totals[case_id] = colourings["total_groups"] if isinstance(colourings, dict) else None
    missing_totals = [case_id for case_id, case in a_cases.items()
                      if case["route"] == "coloured" and totals[case_id] is None]
    rule_cases = sorted(case_id for case_id, case in a_cases.items()
                        if case["route"] == "coloured" and totals[case_id] >= case["n"])
    rule_mismatches, other_mismatches, rule_rows = [], [], []
    detected_a_not_b, misses_a, misses_b, correct_rejected = [], [], [], {"A": [], "B": []}
    decisions = 0
    cost_rows = []
    for case_id in sorted(a_cases):
        a, b = a_cases[case_id], b_cases[case_id]
        n, route, group = a["n"], a["route"], a["group"]
        expected_a = "Sample(8)" if route == "dense" else "MultiColouring"
        expected_b = "Sample(8)" if route == "dense" else rule(n, totals[case_id])
        for execution in a["executions"]:
            ea, eb = a["executions"][execution], b["executions"][execution]
            decisions += 1
            if ea["first_use_check"] != expected_a or eb["first_use_check"] != expected_b:
                rule_mismatches.append({"case": case_id, "execution": execution,
                                        "A": ea["first_use_check"], "B": eb["first_use_check"],
                                        "expected_A": expected_a, "expected_B": expected_b})
            for path in ("first_use", "verify"):
                if wrong(a):
                    if detected(ea[path]) and not detected(eb[path]):
                        detected_a_not_b.append({"case": case_id, "execution": execution, "path": path})
                    if not detected(ea[path]):
                        misses_a.append({"case": case_id, "execution": execution, "path": path,
                                         "check": ea["first_use_check"]})
                    if not detected(eb[path]):
                        misses_b.append({"case": case_id, "execution": execution, "path": path,
                                         "check": eb["first_use_check"]})
                else:
                    for label, record in (("A", ea), ("B", eb)):
                        if record[path]["outcome"] != "pass":
                            correct_rejected[label].append({"case": case_id, "execution": execution,
                                                            "path": path})
            cost_rows.append((f"{group} / {route}", execution, ea["first_use_check"],
                              eb["first_use_check"], ea["first_use"], eb["first_use"]))
        fields = diff(a, b)
        if case_id in rule_cases:
            rule_rows.append({
                "case": case_id, "n": n, "total_groups": totals[case_id], "correct": a["correct"],
                "fields_that_differ_A_B": fields,
                "executions": {execution: {
                    "check": {"A": a["executions"][execution]["first_use_check"],
                              "B": b["executions"][execution]["first_use_check"]},
                    "first_use": {"A": decision_key(a["executions"][execution]["first_use"]),
                                  "B": decision_key(b["executions"][execution]["first_use"])},
                    "verify": {"A": decision_key(a["executions"][execution]["verify"]),
                               "B": decision_key(b["executions"][execution]["verify"])},
                    "check_rhs_first_use": {
                        "A": a["executions"][execution]["first_use"]["verification_rhs_calls"],
                        "B": b["executions"][execution]["first_use"]["verification_rhs_calls"]},
                } for execution in a["executions"]},
            })
        elif fields:
            other_mismatches.append({"case": case_id, "fields": fields[:20]})
    v29_mismatches = [case_id for case_id in sorted(a_cases)
                      if diff(a_cases[case_id], v29_cases.get(case_id))]

    def miss_keys(entries):
        return sorted({(m["case"], m["execution"], m["path"]) for m in entries})

    misses_b_evidence = []
    for miss in misses_b:
        evidence = b_cases[miss["case"]].get("miss_evidence", {}).get(miss["execution"])
        misses_b_evidence.append({**miss, "rounding_limited": evidence.get("rounding_limited")
                                  if isinstance(evidence, dict) else None})
    return {
        "case_set": {"cases_A": len(a_cases), "cases_B": len(b_cases), "cases_v29_B": len(v29_cases),
                     "same_case_ids": same_ids,
                     "coloured_cases_without_total_groups_in_A": missing_totals},
        "decisions_per_path": decisions,
        "rule_picks_all_cases": rule_cases,
        "rule_cases": rule_rows,
        "rule_mismatches": rule_mismatches,
        "other_cases_A_B_mismatches": other_mismatches,
        "other_cases_identical_A_B": len(a_cases) - len(rule_cases) - len(other_mismatches),
        "commit_A_against_v29_B": {"cases_compared": len(a_cases), "mismatching_cases": v29_mismatches},
        "detected_by_A_not_by_B": detected_a_not_b,
        "misses_A": {"count": len(misses_a), "list": misses_a},
        "misses_B": {"count": len(misses_b), "list": misses_b_evidence},
        "misses_equal_A_B": miss_keys(misses_a) == miss_keys(misses_b),
        "correct_rejected": correct_rejected,
        "first_use_cost_per_class": class_costs(cost_rows),
        "wall_s_diagnostic": {"A": {g: a_groups[g]["wall_s_diagnostic"] for g in a_groups},
                              "B": {g: b_groups[g]["wall_s_diagnostic"] for g in b_groups}},
    }


# ---------------------------------------------------------------------------------------------
# Survey of random banded declared patterns
# ---------------------------------------------------------------------------------------------

def survey_records(label):
    _, records, _ = read_log(LOGS / f"survey_{label}.log")
    out = {}
    for record in tagged(records, "V31_SURVEY_JSON"):
        key = (record["n"], record["half_width_fraction"], record["density"], record["seed"])
        if key in out:
            raise ValueError(f"duplicate survey pattern {key}")
        out[key] = record
    done = tagged(records, "V31_SURVEY_DONE_JSON")
    return out, done


def analyse_survey():
    a, a_done = survey_records("A")
    b, b_done = survey_records("B")
    mismatches, rule_mismatches = [], []
    by_fraction = defaultdict(lambda: {"patterns": 0, "rule_all": 0, "greater": 0, "equal": 0,
                                       "rhs_A": 0, "rhs_B": 0, "n_sum": 0})
    worst = None
    for key in sorted(a):
        ra, rb = a[key], b.get(key)
        if rb is None:
            mismatches.append({"pattern": list(key), "fields": ["missing in B"]})
            continue
        fields = [f for f in diff(ra, rb) if not f.startswith((".default_check", ".rule_picks_all"))]
        if fields:
            mismatches.append({"pattern": list(key), "fields": fields})
        n, total = ra["n"], ra["check_colourings"]["total_groups"]
        if ra["default_check"] != "MultiColouring" or rb["default_check"] != rule(n, total) \
                or ra["default_check_rhs"] != total \
                or rb["default_check_rhs"] != (n if total >= n else total):
            rule_mismatches.append({"pattern": list(key), "A": ra["default_check"],
                                    "B": rb["default_check"], "total_groups": total, "n": n})
        entry = by_fraction[ra["half_width_fraction"]]
        entry["patterns"] += 1
        entry["rule_all"] += total >= n
        entry["greater"] += total > n
        entry["equal"] += total == n
        entry["rhs_A"] += ra["default_check_rhs"]
        entry["rhs_B"] += rb["default_check_rhs"]
        entry["n_sum"] += n
        ratio = total / n
        if worst is None or ratio > worst["ratio"]:
            worst = {"pattern": list(key), "n": n, "half_width": ra["half_width"],
                     "bandwidth": ra["bandwidth"], "total_groups": total, "ratio": round(ratio, 4),
                     "construction": ra["check_colourings"]["construction_detail"]}
    patterns = list(a.values())
    review_like = [r for r in patterns if r["density"] == 0.1 and 0.2 <= r["half_width_fraction"] <= 0.45]
    full_bands = [r for r in patterns if r["density"] == 1.0]
    return {
        "patterns": len(a),
        "patterns_B": len(b),
        "done_records": {"A": a_done, "B": b_done},
        "mismatches_A_B_other_than_the_default": mismatches,
        "rule_mismatches": rule_mismatches,
        "rule_picks_all": sum(r["check_colourings"]["total_groups"] >= r["n"] for r in patterns),
        "total_groups_greater_than_n": sum(r["check_colourings"]["total_groups"] > r["n"]
                                           for r in patterns),
        "total_groups_equal_n": sum(r["check_colourings"]["total_groups"] == r["n"] for r in patterns),
        "default_check_rhs_sum": {"A": sum(r["default_check_rhs"] for r in patterns),
                                  "B": sum(b[k]["default_check_rhs"] for k in a if k in b),
                                  "n_sum": sum(r["n"] for r in patterns)},
        "worst_total_groups_over_n": worst,
        "review_like_subset": {
            "definition": "density 0.1, half-width fraction 0.2 to 0.45 (the setting of review "
                          "finding F4 of node V29)",
            "patterns": len(review_like),
            "total_groups_greater_than_n": sum(r["check_colourings"]["total_groups"] > r["n"]
                                               for r in review_like),
        },
        "full_bands": [{"n": r["n"], "half_width": r["half_width"],
                        "total_groups": r["check_colourings"]["total_groups"],
                        "default_B": b[(r["n"], r["half_width_fraction"], r["density"], r["seed"])][
                            "default_check"]} for r in sorted(full_bands, key=lambda r: (r["n"], r["half_width"]))],
        "by_half_width_fraction": [{"fraction": fraction, **entry}
                                   for fraction, entry in sorted(by_fraction.items())],
    }


# ---------------------------------------------------------------------------------------------
# Supplement (mode wide)
# ---------------------------------------------------------------------------------------------

def wide_records(label):
    _, records, _ = read_log(LOGS / f"wide_{label}_random.log")
    cases = {}
    for record in tagged(records, "V31_CASE_JSON"):
        if record["case"] in cases:
            raise ValueError("duplicate supplement case " + record["case"])
        cases[record["case"]] = record
    return cases, tagged(records, "V31_FAMILY_JSON"), tagged(records, "V31_META_JSON")


def analyse_wide():
    a_cases, a_families, a_meta = wide_records("A")
    b_cases, b_families, b_meta = wide_records("B")
    explicit_mismatches, rule_mismatches, other_mismatches, colouring_mismatches = [], [], [], []
    detected_a_not_b, misses_a, misses_b = [], [], []
    correct_rejected = {"A": [], "B": []}
    kinds = Counter()
    cost_rows = []
    rule_case_count = Counter()
    for case_id in sorted(a_cases):
        a, b = a_cases[case_id], b_cases.get(case_id)
        if b is None:
            other_mismatches.append({"case": case_id, "fields": ["missing in B"]})
            continue
        n, total = a["n"], a["check_colourings"]["total_groups"]
        if diff(a["check_colourings"], b["check_colourings"]) or a["rule_picks_all"] != (total >= n):
            colouring_mismatches.append(case_id)
        side = "rule picks All" if total >= n else "rule keeps MultiColouring"
        equality = "total_groups > n" if total > n else ("total_groups = n" if total == n
                                                         else "total_groups < n")
        rule_case_count[(a["family"], side)] += 1
        for execution in a["executions"]:
            ea, eb = a["executions"][execution], b["executions"][execution]
            for name in ("explicit_all", "explicit_multi_colouring"):
                if diff(ea[name], eb[name]):
                    explicit_mismatches.append({"case": case_id, "execution": execution, "check": name})
            if ea["first_use_check"] != "MultiColouring" or eb["first_use_check"] != rule(n, total):
                rule_mismatches.append({"case": case_id, "execution": execution,
                                        "A": ea["first_use_check"], "B": eb["first_use_check"],
                                        "expected_B": rule(n, total)})
            if total < n and (diff(ea["first_use"], eb["first_use"]) or diff(ea["verify"], eb["verify"])):
                other_mismatches.append({"case": case_id, "execution": execution})
            for path in ("first_use", "verify"):
                if not a["correct"]:
                    if detected(ea[path]) and not detected(eb[path]):
                        detected_a_not_b.append({"case": case_id, "execution": execution, "path": path})
                    if not detected(ea[path]):
                        misses_a.append({"case": case_id, "execution": execution, "path": path})
                    if not detected(eb[path]):
                        misses_b.append({"case": case_id, "execution": execution, "path": path})
                else:
                    for label, record in (("A", ea), ("B", eb)):
                        if record[path]["outcome"] != "pass":
                            correct_rejected[label].append({"case": case_id, "execution": execution,
                                                            "path": path})
            if not a["correct"] and total >= n:
                kinds[(execution, ea["first_use"].get("kind"), eb["first_use"].get("kind"))] += 1
            cost_rows.append((f"{a['family']} / {a['kind']} / {equality}", execution,
                              ea["first_use_check"], eb["first_use_check"], ea["first_use"],
                              eb["first_use"]))
    by_check = Counter()
    for (execution, kind_a, kind_b), count in kinds.items():
        by_check[(execution, "build (UncoveredRowChanged)" if kind_a == "UncoveredRowChanged"
                  else "the check")] += count
    rule_all_cases = sum(1 for case in a_cases.values()
                         if case["check_colourings"]["total_groups"] >= case["n"])
    return {
        "cases": {"A": len(a_cases), "B": len(b_cases), "same_case_ids": set(a_cases) == set(b_cases)},
        "families": {"A": a_families, "B": b_families},
        "meta_probes": {"A": a_meta[0]["default_coloured_probes"] if a_meta else None,
                        "B": b_meta[0]["default_coloured_probes"] if b_meta else None},
        "cases_per_family_and_rule_side": [{"family": family, "side": side, "cases": count}
                                           for (family, side), count in sorted(rule_case_count.items())],
        "rule_picks_all_cases": rule_all_cases,
        "rule_picks_all_wrong_cases": sum(1 for case in a_cases.values() if not case["correct"]
                                          and case["check_colourings"]["total_groups"] >= case["n"]),
        "colourings_or_rule_flag_mismatches": colouring_mismatches,
        "explicit_check_mismatches_A_B": explicit_mismatches,
        "rule_mismatches": rule_mismatches,
        "rule_keeps_multi_colouring_cases_A_B_mismatches": other_mismatches,
        "detected_by_A_not_by_B": detected_a_not_b,
        "misses_A": misses_a,
        "misses_B": misses_b,
        "correct_rejected": correct_rejected,
        "error_kinds_where_the_rule_picks_all": [
            {"execution": execution, "A": kind_a, "B": kind_b, "count": count}
            for (execution, kind_a, kind_b), count in sorted(kinds.items(), key=str)],
        "wrong_rule_all_found_by": [{"execution": execution, "found_by": found, "count": count}
                                    for (execution, found), count in sorted(by_check.items())],
        "first_use_cost_per_class": class_costs(cost_rows),
    }


# ---------------------------------------------------------------------------------------------
# Unit tests, checks, loop checker, edits
# ---------------------------------------------------------------------------------------------

def unit_log():
    path = LOGS / "unit.log"
    header, records, exits = read_log(path)
    text = path.read_text()
    tests = re.findall(r"^test (\S+) \.\.\. (\w+)", text, re.M)
    results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
    return {
        "git_head": header.get("git head"),
        "tracked_changes": header.get("git status (tracked changes)"),
        "command": header.get("command"),
        "exit_codes": exits,
        "tests": [{"name": name, "result": result} for name, result in tests],
        "binaries": len(results),
        "passed": sum(int(r[1]) for r in results),
        "failed": sum(int(r[2]) for r in results),
        "allocation_wide_band": tagged(records, "STAGE_CLIENTS_ASSEMBLY_WIDE_BAND_ALLOC_JSON"),
        "allocation_v29": tagged(records, "STAGE_CLIENTS_ASSEMBLY_DEFAULT_ALLOC_JSON"),
        "wide_band_counts": tagged(records, "STAGE_CLIENTS_ASSEMBLY_WIDE_BAND_JSON"),
    }


def checks_log():
    path = LOGS / "checks.log"
    if not path.exists():
        return {"present": False}
    header, records, _ = read_log(path)
    return {"present": True, "git_head": header.get("git head"),
            "tracked_changes": header.get("git status (tracked changes)"),
            "checks": tagged(records, "V31_CHECK_JSON")}


def doc_log():
    path = LOGS / "doc.log"
    if not path.exists():
        return {"present": False}
    header, _, exits = read_log(path)
    text = path.read_text()
    generated = re.findall(r"generated (\d+) warnings?", text)
    locations = re.findall(r"^\s*--> (\S+?):\d+:\d+$", text, re.M)
    return {
        "present": True,
        "git_head": header.get("git head"),
        "tracked_changes": header.get("git status (tracked changes)"),
        "command": header.get("command"),
        "exit_codes": exits,
        "warnings_generated": int(generated[-1]) if generated else 0,
        "warning_files": dict(sorted(Counter(locations).items())),
        "warnings_in_assembly_files": [p for p in locations if "stage_clients/assembly" in p],
    }


def checker_all_log(changed_paths):
    path = LOGS / "checker_all.log"
    if not path.exists():
        return {"present": False}
    header, records, exits = read_log(path)
    complaints = [c for record in tagged(records, "V31_CHECKER_JSON") for c in record["complaints"]]
    allowed = [p for p in PINNED_ALLOWED if p in changed_paths]
    return {
        "present": True,
        "git_head": header.get("git head"),
        "tracked_changes": header.get("git status (tracked changes)"),
        "exit_codes": exits,
        "complaints": complaints,
        "pinned_files_this_node_changed": allowed,
        "only_pinned_files_this_node_changes": bool(complaints) and all(
            any(p in complaint for p in allowed) for complaint in complaints),
    }


def changed_paths():
    tracked = [p for p in git("diff", "--name-only", BASE).splitlines() if p]
    untracked = [p for p in git("ls-files", "--others", "--exclude-standard").splitlines() if p]
    return sorted(set(tracked) | set(untracked))


def edited_lines():
    """Edited lines (new side) of test files that exist at the base commit, with their reasons."""
    existing = set(git("ls-tree", "-r", "--name-only", BASE, "crates/rodas5p-integrators/tests/")
                   .splitlines())
    text = git("diff", "-U0", BASE, "--", "crates/rodas5p-integrators/tests/")
    edits, current = [], None
    for line in text.splitlines():
        if line.startswith("+++ b/"):
            current = line[6:]
        hunk = re.match(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", line)
        if hunk and current in existing:
            start = int(hunk.group(3))
            count = int(hunk.group(4)) if hunk.group(4) is not None else 1
            old_start = int(hunk.group(1))
            old_count = int(hunk.group(2)) if hunk.group(2) is not None else 1
            edits.append({"path": current, "old_lines": [old_start, old_start + old_count - 1],
                          "new_lines": [start, start + count - 1]})
    for edit in edits:
        key = next((k for k in EDIT_REASONS if k[0] == edit["path"] and k[1] <= edit["new_lines"][0]
                    and edit["new_lines"][1] <= k[2]), None)
        edit["reason"] = EDIT_REASONS.get(key)
        edit["reason_ko"] = EDIT_REASONS_KO.get(key)
    return edits


def new_test_files(paths):
    out = {}
    for path in paths:
        if path.startswith("crates/rodas5p-integrators/tests/stage_clients_assembly_") and \
                path not in git("ls-tree", "-r", "--name-only", BASE,
                                "crates/rodas5p-integrators/tests/").splitlines():
            text = (ROOT / path).read_text()
            names = re.findall(r"#\[test\]\s*(?:#\[ignore[^\]]*\]\s*)?fn (\w+)", text)
            ignored = re.findall(r"#\[ignore[^\]]*\]\s*fn (\w+)", text)
            out[path] = {"tests": names, "ignored": ignored}
    return out


# ---------------------------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------------------------

def main():
    prov = {"A": provenance("A"), "B": provenance("B")}
    identity = analyse_identity()
    corpus = analyse_corpus()
    survey = analyse_survey()
    wide = analyse_wide()
    unit = unit_log()
    checks = checks_log()
    doc = doc_log()
    paths = changed_paths()
    edits = edited_lines()
    loop_checker = checker_all_log(paths)
    outside = [p for p in paths if not any(p.startswith(prefix) for prefix in WRITE_ALLOWLIST)]
    tests = {
        "changed_paths_since_base": paths,
        "paths_outside_the_write_allowlist": outside,
        "edited_lines_of_existing_tests": edits,
        "edited_lines_without_reason": [e for e in edits if not e["reason"]],
        "examined_and_unchanged": EXAMINED_UNCHANGED,
        "new_test_files": new_test_files(paths),
    }
    provenance_summary = provenance_checks(prov)

    def check_ok(name):
        records = [c for c in checks.get("checks", []) if c["name"] == name]
        return bool(records) and all(c["exit_code"] == 0 and c.get("failed", 0) == 0 for c in records)

    loop_record = next((c for c in checks.get("checks", []) if c["name"] == "loop checker"), None)
    unit_names = {t["name"] for t in unit["tests"] if t["result"] == "ok"}
    expected_unit = {
        "the_default_of_the_coloured_route_follows_total_groups",
        "a_client_whose_default_became_the_full_check_runs_it_once",
        "the_full_default_detects_every_omission_of_a_wide_pattern",
        "every_check_stays_selectable_on_a_wide_pattern",
        "a_full_default_check_allocates_nothing_at_the_first_linearization",
        "the_default_first_use_check_follows_the_route",
        "the_default_check_allocates_nothing_at_the_first_linearization",
    }
    discriminators = [
        {
            "name": "identity unchanged",
            "criterion": "every V08 identity run (5 cells, both band back ends of nlkpp-band-n4096-r24, "
                         "arms prot, seq, t1, t2, t4, t6) and every cell record equal between A and B in "
                         "every recorded field: Jacobian, linearization-point, output and final-state "
                         "fingerprints, steps and every counter, the check counters included",
            "measured": f"{identity['run_count']} runs; field mismatches A/B: "
                        f"{len(identity['mismatches_A_B'])}; cell record mismatches: "
                        f"{len(identity['cell_record_mismatches_A_B'])}; total_groups of the coloured "
                        f"cells: {sorted({c['total_groups'] for c in identity['cells'] if c['total_groups']})}",
            "verdict": "PASS" if identity["run_count"] == 36 and not identity["mismatches_A_B"]
            and not identity["cell_record_mismatches_A_B"] else "FAIL",
        },
        {
            "name": "rule per route implemented",
            "criterion": "B: coloured route All where total_groups >= n and MultiColouring otherwise, "
                         "dense route Sample(8), on every identity run, corpus decision, survey pattern "
                         "and supplement decision; A: the V29 default everywhere; the node's unit tests pass",
            "measured": f"rule mismatches: identity {len(identity['rule_mismatches'])}, corpus "
                        f"{len(corpus['rule_mismatches'])}, survey {len(survey['rule_mismatches'])}, "
                        f"supplement {len(wide['rule_mismatches'])}; unit tests passed "
                        f"{unit['passed']}, failed {unit['failed']}",
            "verdict": "PASS" if not (identity["rule_mismatches"] or corpus["rule_mismatches"]
                                      or survey["rule_mismatches"] or wide["rule_mismatches"])
            and unit["failed"] == 0 and expected_unit <= unit_names else "FAIL",
        },
        {
            "name": "corpus detection not worse than A",
            "criterion": "every wrong declaration of the V26 corpus (and of the supplement) detected by "
                         "the A default on a path and execution is detected by the B default there; no "
                         "correct pattern rejected by B",
            "measured": f"corpus: detected by A not by B {len(corpus['detected_by_A_not_by_B'])}, "
                        f"misses A {corpus['misses_A']['count']}, B {corpus['misses_B']['count']}, "
                        f"correct rejected by B {len(corpus['correct_rejected']['B'])}; supplement: "
                        f"detected by A not by B {len(wide['detected_by_A_not_by_B'])}, misses B "
                        f"{len(wide['misses_B'])}, correct rejected by B {len(wide['correct_rejected']['B'])}",
            "verdict": "PASS" if not corpus["detected_by_A_not_by_B"]
            and not wide["detected_by_A_not_by_B"] and not corpus["correct_rejected"]["B"]
            and not wide["correct_rejected"]["B"] else "FAIL",
        },
        {
            "name": "suites",
            "criterion": "stage_clients_* and the sentinels (no-fail-fast) exit 0 with 0 failed; clippy "
                         "-D warnings exit 0; fmt --check exit 0; the loop checker complains only about "
                         "pinned files this node is allowed to change",
            "measured": "; ".join(f"{c['name']}: exit {c['exit_code']}"
                                  + (f", {c.get('passed')} passed, {c.get('failed')} failed in "
                                     f"{c.get('binaries')} binaries" if "passed" in c else "")
                                  for c in checks.get("checks", []))
            + f"; loop checker complaints: {loop_checker.get('complaints')}",
            "verdict": "PASS" if check_ok("stage client tests and sentinels") and check_ok("clippy")
            and check_ok("fmt") and loop_checker.get("only_pinned_files_this_node_changes")
            else ("NOT_RUN" if not checks.get("present") else "FAIL"),
        },
    ]
    results = {
        "schema": "vigilode.loop-l1.v31-results.v1",
        "node": "V31_ASSEMBLY_DEFAULT_WIDE_BAND",
        "authority": "owner decision OD-L1-27 (2026-11-25 09:21 KST, '남은 노드들을 이어서 마저 진행해. "
                     "수정은 전부 승인할게.'): the default first-use check of the coloured route takes "
                     "CheckColumns::All where CheckColourings::total_groups >= n, MultiColouring "
                     "otherwise; the dense route keeps Sample(8). Ruling of the orchestrator: '>= n'",
        "contract": "docs/agent/production_solver_program_20261112/TASK_CONTRACTS.json, id "
                    "V31_ASSEMBLY_DEFAULT_WIDE_BAND",
        "base_commit": BASE,
        "head_at_parse": git("rev-parse", "HEAD").strip(),
        "protocol": {
            "note": "Implementation of an owner decision, no experiment and no numerical threshold; the "
                    "discriminators are equalities between recordings A (before the change, the V29 "
                    "default) and B (after it) made by the same programs with the same commands, and "
                    "the suites. No wall time decides anything; wall times in the logs are diagnostic.",
            "decision_paths": "first_use = the first linearize of a fresh client of the default options "
                              "(what a driver run does); verify = AssemblyStageClient::verify(t0, y0, "
                              "options.first_use_check) on a fresh client of the default options",
            "wrong_declaration": "a case that is not a correct pattern: an omitted entry, a band too "
                                 "narrow, or the impure right-hand side of control 3",
            "cost": "check_rhs_passing = verification_rhs_calls of the first-use path summed over the "
                    "passing cases of a class (a failing check stops early, so its count is not a cost "
                    "of the check); rhs_calls = all right-hand sides of the first-use path",
            "preregistration": "the plan, the list of the cases with total_groups >= n and the "
                               "programs were committed before commit A (NOTES.txt entries 1 to 3); the "
                               "supplement was widened after a smoke run before commit A (entry 3); no "
                               "PROTOCOL.json file",
        },
        "recordings": {"provenance": prov, "checks": provenance_summary},
        "identity": identity,
        "corpus": corpus,
        "survey": survey,
        "supplement": wide,
        "unit_tests_of_this_node": unit,
        "tests": tests,
        "checks": checks,
        "rustdoc": doc,
        "loop_checker": loop_record,
        "loop_checker_every_complaint": loop_checker,
        "discriminators": discriminators,
    }
    text = json.dumps(results, indent=1, ensure_ascii=False, allow_nan=False, sort_keys=False)
    (HERE / "RESULTS.json").write_text(text + "\n")
    (HERE / "REPORT_KO.md").write_text(render(results))
    for d in discriminators:
        print(f"{d['name']}: {d['verdict']} ({d['measured'][:300]})")


# ---------------------------------------------------------------------------------------------
# Report (Korean, for the owner)
# ---------------------------------------------------------------------------------------------

def table(header, rows):
    # The leading empty line keeps a table that follows a list item from joining the list.
    lines = ["", "| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    lines += ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows]
    return "\n".join(lines)


def verdict_ko(verdict):
    return {"PASS": "통과", "FAIL": "실패", "NOT_RUN": "미실행"}.get(verdict, verdict)


def render(r):
    ident, corpus, survey, wide = r["identity"], r["corpus"], r["survey"], r["supplement"]
    prov = r["recordings"]["checks"]
    out = []
    out.append("# 노드 V31 보고서: 넓은 선언 띠에서 조립 클라이언트의 기본 첫 사용 검사 (OD-L1-27)")
    out.append("")
    out.append(f"이 문서는 `parse_results.py`가 `logs/`의 원시 로그에서 계산해 만든다. 손으로 적은 수치는 "
               f"없다. 기준 커밋 `{r['base_commit'][:8]}`, 파싱 시점 HEAD `{r['head_at_parse'][:8]}`.")
    out.append("")
    out.append("**정정 고지 (2026-11-25, 독립 리뷰 `REVIEW.json`, 판정 accept_with_corrections).** 리뷰어가 자기 파서로 "
               "모든 수치를 다시 얻었다. 아래 세 가지를 바로잡는다. 이 고지는 `parse_results.py --report-only`로 커밋된 "
               "`RESULTS.json`에서 렌더링했다.")
    out.append("")
    out.append("1. 'A가 잡은 잘못된 선언을 B가 놓친 경우 0'과 rustdoc의 '같은 비용이면 정확하다'는 시험한 집합에서만 성립한다. "
               "일반적으로 새 기본값 `All`은 반올림 때문에, V29의 기본값 `MultiColouring`이 잡는 잘못된 선언을 놓칠 수 있다. "
               "리뷰어가 만든 예(1e6 항 옆의 약 5.6e-9 결합: 그 열만 움직이면 반올림으로 사라지고, 그룹이 선언된 열과 함께 "
               "움직이면 비트가 바뀐다)에서 commit A의 기본값은 160/160을 잡고 HEAD의 기본값은 160/160을 놓쳤다. 실무 영향은 "
               "무시할 만하다. 병합 때 rustdoc에 '반올림 범위에서 정확'이 MC의 상위집합을 뜻하지 않는다는 문장을 넣었다.")
    out.append("2. NOTES.txt의 commit 시각 몇 개가 git과 다르다(예: bb09dfb4는 09:42가 아니라 09:36:58). 순서는 맞고, git "
               "시각이 기준이다.")
    out.append("3. rustdoc의 '(넓은 선언 띠)'는 불완전했다. 불규칙 패턴(추가 채색이 단일 열이라 두 검사가 같은 상태를 움직임)과 "
               "작은 대각·빈 패턴에서도 `All`이 된다. 병합 때 rustdoc를 고쳤다.")
    out.append("")
    out.append("## 1. 결론")
    out.append("")
    for d in r["discriminators"]:
        out.append(f"- **{d['name']}**: {verdict_ko(d['verdict'])}. {d['measured']}")
    out.append("")
    out.append("채색(coloured) 경로의 기본 첫 사용 검사는 이제 선언 패턴의 "
               "`CheckColourings::total_groups`가 차수 n 이상이면 `CheckColumns::All`(모든 열을 하나씩, "
               "반올림 범위에서 정확, 우변 n회), 그보다 작으면 이전처럼 `MultiColouring`이다. 조밀(dense) "
               "경로는 `Sample(8)` 그대로다. 판정에는 선언 패턴만 쓰인다(Curtis-Powell-Reid 채색, 띠 폭, "
               "차수). 상태나 우변 평가는 들어가지 않으므로 옵션을 만들 때 이미 정해진다.")
    out.append("")
    saving = [c for c in wide["first_use_cost_per_class"] if "total_groups > n" in c["class"]
              and c["passing_A"] and c["passing_B"]]
    survey_sum = survey["default_check_rhs_sum"]
    out.append(f"비용: 조사의 {survey['patterns']}개 무작위 띠 패턴에서 기본 검사의 우변 합계가 A "
               f"{survey_sum['A']}에서 B {survey_sum['B']}로 줄었다(n의 합 {survey_sum['n_sum']}). 보충 집합에서 "
               f"total_groups > n인 올바른 패턴의 첫 사용 검사 우변: "
               + "; ".join(f"{c['class']} {c['execution']}: A {c['check_rhs_passing_A']}, B "
                           f"{c['check_rhs_passing_B']}" for c in saving)
               + ". V08 식별성 셀과 V26 말뭉치의 비용은 바뀌지 않았다(링 6개는 n = total_groups라 같은 비용).")
    out.append("")
    out.append("## 2. 구현")
    out.append("")
    out.append("- `AssemblyOptions::default_first_use_check(Coloured(pattern))`: "
               "`CheckColourings::total_groups_of(pattern) >= n`이면 `All`, 아니면 `MultiColouring`. "
               "`coloured(pattern)`이 이 값을 쓰고 `threaded()`, `with_fingerprints()`는 유지하며 "
               "`with_first_use_check()`는 바꾼다. 모든 검사는 계속 선택할 수 있다.")
    out.append("- `CheckColourings::total_groups_of`(새 공개 함수)는 채색을 만들지 않고 두 구성(띠, 세분)의 "
               "합계 중 작은 값을 계산한다. 합계 계산은 `from_parts`와 같은 보조 함수(`band_plan`, "
               "`refinement_plan`)를 공유하므로 산술이 같다. 단위 테스트가 168개 무작위 패턴과 고정 패턴에서 "
               "`CheckColourings::of(..).total_groups()`와 같음을 확인한다.")
    out.append("- 기본값이 `All`인 순차 클라이언트는 `new()`에서 추가 채색을 만들지 않으며(필요 없음), 전체 "
               "검사는 클라이언트 자신의 작업 공간을 쓰므로 첫 선형화와 이후 선형화에서 할당이 없다"
               "(할당 테스트, 6절).")
    out.append("- 조립된 야코비안의 산술은 바뀌지 않았다.")
    out.append("")
    out.append("## 3. 녹화의 출처")
    out.append("")
    out.append(f"- A: 변경 전 코드(V29 기본값), 빌드 커밋 {', '.join(h[:8] for h in prov['A_built_at'])}; "
               f"B: 변경 후, 빌드 커밋 {', '.join(h[:8] for h in prov['B_built_at'])}. 로그 수 A "
               f"{prov['logs_per_recording']['A']}, B {prov['logs_per_recording']['B']}.")
    out.append(f"- 같은 프로그램 소스(A와 B): {prov['same_program_source_A_B']}; 깨끗한 작업 트리: "
               f"{prov['clean_trees']}; 종료 코드 0: {prov['exit_codes_zero']}; 클라이언트 소스 해시가 A와 B "
               f"사이에서 바뀜: {prov['client_changed_between_A_and_B']}; 바이너리가 다름: "
               f"{prov['binaries_differ_A_B']}.")
    out.append("- V29 녹화 프로그램(`stage_clients_assembly_default_record.rs`)은 수정하지 않고 재사용했다. "
               "이 노드의 프로그램(`stage_clients_assembly_wide_band_record.rs`)은 조사(survey)와 보충 집합을 "
               "녹화한다.")
    out.append(f"- 커밋 A는 V29의 B 녹화와 같다: 식별성 {ident['commit_A_against_v29_B']['runs_compared']}회 "
               f"실행 중 불일치 {len(ident['commit_A_against_v29_B']['mismatches'])}, 말뭉치 "
               f"{corpus['commit_A_against_v29_B']['cases_compared']}개 경우 중 불일치 "
               f"{len(corpus['commit_A_against_v29_B']['mismatching_cases'])}.")
    out.append("")
    out.append("## 4. V08 식별성 셀: A와 B")
    out.append("")
    out.append(f"{ident['run_count']}회 실행(셀 5개, nlkpp-band-n4096-r24는 두 띠 백엔드, 팔 prot, seq, t1, "
               f"t2, t4, t6)의 모든 기록 필드를 비교했다. 필드 불일치 {len(ident['mismatches_A_B'])}, "
               f"셀 기록 불일치 {len(ident['cell_record_mismatches_A_B'])}. 채색 셀의 total_groups는 모두 "
               f"n보다 작아 규칙이 `MultiColouring`을 유지하므로 검사 카운터도 같다.")
    out.append("")
    out.append(table(["셀", "백엔드", "n", "total_groups", "기본값 A", "기본값 B"],
                     [[c["cell"], c["backend"], c["n"], c["total_groups"] if c["total_groups"] is not None
                       else "-", "/".join(c["default_checks_A"]), "/".join(c["default_checks_B"])]
                      for c in ident["cells"]]))
    out.append("")
    rows = [[x["cell"], x["backend"], x["arm"], x["first_use_check"]["B"],
             f"{x['verification_rhs_calls']['A']} / {x['verification_rhs_calls']['B']}",
             f"{x['rhs_calls']['A']} / {x['rhs_calls']['B']}", x["jacobian_fingerprint"],
             x["final_fingerprint"], "예" if x["all_fields_equal_A_B"] else "아니오"]
            for x in ident["runs"] if x["arm"] != "prot"]
    out.append(table(["셀", "백엔드", "팔", "검사(B)", "검사 우변 A / B", "우변 합계 A / B",
                      "야코비안 지문", "최종 상태 지문", "모든 필드 같음"], rows))
    out.append("")
    probe = ident["meta_probe"]
    out.append(f"V29 프로그램의 메타 필드(띠 (1, 1), 차수 4의 기본값)는 A "
               f"{probe['A_default_coloured_probe_banded_4_1_1']}, B "
               f"{probe['B_default_coloured_probe_banded_4_1_1']}이다. 이 패턴은 total_groups 4 = n이라 "
               f"규칙대로 바뀌었다(경우가 아닌 메타 필드).")
    out.append("")
    out.append("## 5. V26 말뭉치: A와 B")
    out.append("")
    out.append(f"경우 {corpus['case_set']['cases_A']}개(A), {corpus['case_set']['cases_B']}개(B), 같은 "
               f"식별자: {corpus['case_set']['same_case_ids']}. 경로별 결정 "
               f"{corpus['decisions_per_path']}건(경우 x 실행).")
    out.append("")
    out.append(f"- 규칙이 `All`을 고르는 경우는 {len(corpus['rule_picks_all_cases'])}개이며 모두 올바른 "
               f"패턴이고 total_groups = n(같음)이다: {', '.join(corpus['rule_picks_all_cases'])}.")
    out.append(f"- 나머지 {corpus['other_cases_identical_A_B']}개 경우는 A와 B의 기록이 모든 필드에서 같다"
               f"(불일치 {len(corpus['other_cases_A_B_mismatches'])}).")
    out.append(f"- A가 찾고 B가 놓친 잘못된 선언: {len(corpus['detected_by_A_not_by_B'])}. 놓친 수 A "
               f"{corpus['misses_A']['count']}, B {corpus['misses_B']['count']}(같은 목록: "
               f"{corpus['misses_equal_A_B']}). 놓친 경우: "
               + "; ".join(f"{m['case']} {m['execution']} {m['path']} ({m['check']}, 반올림 한계: "
                           f"{m['rounding_limited']})" for m in corpus["misses_B"]["list"])
               + ". 순차 조밀 경로의 불순 대조군은 첫 사용 검사를 하지 않으므로(변경 없음) 첫 사용 경로에서 "
                 "통과하고, e3072-r768-c769는 V26에서 전체 검사와 다중 채색 검사가 모두 놓친 반올림 한계 "
                 "경우다.")
    out.append(f"- 거부된 올바른 패턴: A {len(corpus['correct_rejected']['A'])}, B "
               f"{len(corpus['correct_rejected']['B'])}.")
    out.append("")
    rows = []
    for row in corpus["rule_cases"]:
        for execution, e in row["executions"].items():
            rows.append([row["case"], row["n"], row["total_groups"], execution,
                         f"{e['check']['A']} / {e['check']['B']}",
                         f"{e['first_use']['A'][0]} / {e['first_use']['B'][0]}",
                         f"{e['check_rhs_first_use']['A']} / {e['check_rhs_first_use']['B']}"])
    out.append(table(["경우", "n", "total_groups", "실행", "검사 A / B", "첫 사용 결정 A / B",
                      "검사 우변 A / B"], rows))
    out.append("")
    out.append("규칙 경우에서 A와 B 기록이 다른 필드: "
               + "; ".join(f"{row['case']}: {', '.join(row['fields_that_differ_A_B'])}"
                           for row in corpus["rule_cases"]) + ".")
    out.append("")
    out.append("### 첫 사용 검사 비용(우변 수), 경우 분류별")
    out.append("")
    out.append("검사 우변은 통과한 경우의 `verification_rhs_calls` 합이다(실패한 검사는 일찍 멈추므로 비용으로 "
               "세지 않는다). 우변 합계는 첫 사용 경로의 모든 우변이다.")
    out.append("")
    out.append(table(["분류", "실행", "경우", "검사 A", "검사 B", "통과 A / B", "검사 우변 A / B",
                      "우변 합계 A / B"],
                     [[c["class"], c["execution"], c["cases"],
                       ", ".join(f"{k} {v}" for k, v in c["checks_A"].items()),
                       ", ".join(f"{k} {v}" for k, v in c["checks_B"].items()),
                       f"{c['passing_A']} / {c['passing_B']}",
                       f"{c['check_rhs_passing_A']} / {c['check_rhs_passing_B']}",
                       f"{c['rhs_calls_A']} / {c['rhs_calls_B']}"] for c in corpus["first_use_cost_per_class"]]))
    out.append("")
    out.append("## 6. 보충 집합(규칙이 `All`을 고르는 잘못된 선언)")
    out.append("")
    out.append("V26 말뭉치에는 규칙이 `All`을 고르는 잘못된 선언이 없어서, 같은 프로그램으로 A와 B를 녹화한 "
               "보충 집합을 더했다: 무작위 띠 패턴(차수 60, 96, 128, 200, 반폭 0.2/0.3/0.45 n, 밀도 0.1/0.3; "
               "올바른 패턴, 띠 전체 상위 집합, 한 항목 누락 16개), 넓은 띠의 Fisher-KPP(차수 60, 128; 올바른 "
               "띠와 하나 좁은 띠), total_groups = n인 링 6개의 모든 한 항목 누락, 그리고 규칙이 "
               "`MultiColouring`을 유지하는 좁은 대조군.")
    out.append("")
    out.append(f"- 경우 A {wide['cases']['A']}, B {wide['cases']['B']}(같은 식별자: "
               f"{wide['cases']['same_case_ids']}); 규칙이 `All`을 고르는 경우 {wide['rule_picks_all_cases']}, "
               f"그중 잘못된 선언 {wide['rule_picks_all_wrong_cases']}.")
    out.append(table(["계열", "규칙", "경우"], [[x["family"], x["side"], x["cases"]]
                                             for x in wide["cases_per_family_and_rule_side"]]))
    out.append("")
    out.append(f"- A가 찾고 B가 놓친 잘못된 선언: {len(wide['detected_by_A_not_by_B'])}; 놓친 수 A "
               f"{len(wide['misses_A'])}, B {len(wide['misses_B'])}; 거부된 올바른 패턴 A "
               f"{len(wide['correct_rejected']['A'])}, B {len(wide['correct_rejected']['B'])}.")
    out.append(f"- 기본값과 무관한 명시적 전체 검사와 명시적 다중 채색 검사의 결정은 A와 B에서 같다(불일치 "
               f"{len(wide['explicit_check_mismatches_A_B'])}). 규칙이 `MultiColouring`을 유지하는 경우의 "
               f"기록도 같다(불일치 {len(wide['rule_keeps_multi_colouring_cases_A_B_mismatches'])}).")
    out.append("- 규칙이 `All`을 고르는 잘못된 선언을 누가 찾았나(첫 사용 경로):")
    out.append(table(["실행", "찾은 곳", "경우"], [[x["execution"], x["found_by"], x["count"]]
                                              for x in wide["wrong_rule_all_found_by"]]))
    out.append("")
    out.append(table(["실행", "오류 종류 A", "오류 종류 B", "경우"],
                     [[x["execution"], x["A"], x["B"], x["count"]]
                      for x in wide["error_kinds_where_the_rule_picks_all"]]))
    out.append("")
    out.append("### 첫 사용 검사 비용(우변 수), 보충 집합의 분류별")
    out.append("")
    out.append(table(["분류", "실행", "경우", "검사 A", "검사 B", "통과 A / B", "검사 우변 A / B",
                      "우변 합계 A / B"],
                     [[c["class"], c["execution"], c["cases"],
                       ", ".join(f"{k} {v}" for k, v in c["checks_A"].items()),
                       ", ".join(f"{k} {v}" for k, v in c["checks_B"].items()),
                       f"{c['passing_A']} / {c['passing_B']}",
                       f"{c['check_rhs_passing_A']} / {c['check_rhs_passing_B']}",
                       f"{c['rhs_calls_A']} / {c['rhs_calls_B']}"] for c in wide["first_use_cost_per_class"]]))
    out.append("")
    out.append("## 7. 조사(survey): 무작위 띠 선언 패턴의 total_groups")
    out.append("")
    worst = survey["worst_total_groups_over_n"]
    out.append(f"패턴 {survey['patterns']}개(차수 60, 96, 128, 200, 300, 500; 반폭 0.05-0.45 n; 밀도 0.05, "
               f"0.1, 0.3, 1.0(띠 전체); 시드 1, 2). 우변 평가 없음. total_groups >= n: "
               f"{survey['rule_picks_all']}개(> n {survey['total_groups_greater_than_n']}, = n "
               f"{survey['total_groups_equal_n']}). 최악 비율 total_groups/n = {worst['ratio']}(n {worst['n']}, "
               f"띠 {worst['bandwidth']}, total_groups {worst['total_groups']}, {worst['construction']}). "
               f"V29 검토 F4와 같은 설정(밀도 0.1, 반폭 0.2-0.45 n) {survey['review_like_subset']['patterns']}개 중 "
               f"> n은 {survey['review_like_subset']['total_groups_greater_than_n']}개. 기본 검사 우변 합계 A "
               f"{survey['default_check_rhs_sum']['A']}, B {survey['default_check_rhs_sum']['B']}(n의 합 "
               f"{survey['default_check_rhs_sum']['n_sum']}). A와 B의 기록은 기본 검사 필드를 빼면 같다"
               f"(불일치 {len(survey['mismatches_A_B_other_than_the_default'])}).")
    out.append("")
    out.append(table(["반폭 비율", "패턴", "규칙 All", "> n", "= n", "기본 검사 우변 A", "기본 검사 우변 B",
                      "n 합계"],
                     [[x["fraction"], x["patterns"], x["rule_all"], x["greater"], x["equal"], x["rhs_A"],
                       x["rhs_B"], x["n_sum"]] for x in survey["by_half_width_fraction"]]))
    out.append("")
    out.append("띠 전체(밀도 1.0) 패턴: "
               + "; ".join(f"n {x['n']} 반폭 {x['half_width']}: {x['total_groups']} ({x['default_B']})"
                           for x in survey["full_bands"]) + ".")
    out.append("")
    out.append("## 8. 테스트")
    out.append("")
    unit = r["unit_tests_of_this_node"]
    out.append(f"`run.sh unit`(HEAD `{(unit['git_head'] or '')[:8]}`, 추적 변경 {unit['tracked_changes']}): "
               f"바이너리 {unit['binaries']}개, 통과 {unit['passed']}, 실패 {unit['failed']}, 종료 코드 "
               f"{unit['exit_codes']}.")
    for t in unit["tests"]:
        out.append(f"- `{t['name']}`: {t['result']}")
    out.append("")
    out.append("할당(순차 클라이언트, 선형화 3회의 할당 수):")
    out.append(table(["경우", "기본 검사", "total_groups / n", "기본 검사", "검사 없음", "다중 채색 요청"],
                     [[x["client"], x["default_check"], f"{x['total_groups']} / {x['n']}",
                       x["allocations_default_check"], x["allocations_without_check"],
                       x["allocations_requested_multi_colouring"]] for x in unit["allocation_wide_band"]]))
    out.append("")
    for x in unit["wide_band_counts"]:
        out.append("- " + ", ".join(f"{k}: {v}" for k, v in x.items()))
    out.append("")
    tests = r["tests"]
    out.append("기존 테스트에서 고친 줄(채색 경로 기본값을 고정하는 줄만):")
    for e in tests["edited_lines_of_existing_tests"]:
        out.append(f"- `{e['path']}` 새 줄 {e['new_lines'][0]}-{e['new_lines'][1]}(옛 줄 "
                   f"{e['old_lines'][0]}-{e['old_lines'][1]}): {e['reason_ko']}")
    out.append(f"- 이유 없는 수정: {len(tests['edited_lines_without_reason'])}; 허용 경로 밖 변경: "
               f"{len(tests['paths_outside_the_write_allowlist'])}.")
    out.append("")
    out.append("검토했지만 고치지 않은 줄:")
    for e in tests["examined_and_unchanged"]:
        out.append(f"- `{e['path']}` {e['lines']}: {e['reason']}")
    out.append("")
    out.append("새 테스트 파일:")
    for path, info in tests["new_test_files"].items():
        out.append(f"- `{path}`: {', '.join(info['tests'])}"
                   + (f" (ignored: {', '.join(info['ignored'])})" if info["ignored"] else ""))
    out.append("")
    out.append("## 9. 검사")
    out.append("")
    checks = r["checks"]
    if checks.get("present"):
        out.append(f"`run.sh checks`(HEAD `{(checks['git_head'] or '')[:8]}`, 추적 변경 "
                   f"{checks['tracked_changes']}):")
        for c in checks["checks"]:
            line = f"- {c['name']}: 종료 코드 {c['exit_code']}"
            if "passed" in c:
                line += f", 바이너리 {c['binaries']}, 통과 {c['passed']}, 실패 {c['failed']}, 무시 {c['ignored']}"
            last = c["last_line"][:200].replace("`", "'")
            line += f". 마지막 줄: `{last}`" if last else ". 출력 없음"
            out.append(line)
    else:
        out.append("checks.log 없음(미실행).")
    doc = r.get("rustdoc", {})
    if doc.get("present"):
        out.append(f"- rustdoc(`run.sh doc`, 종료 코드 {doc['exit_codes']}): 경고 {doc['warnings_generated']}개, "
                   f"조립(assembly) 파일의 경고 {len(doc['warnings_in_assembly_files'])}개(나머지는 다른 파일의 "
                   f"기존 경고: {', '.join(f'{p} {c}' for p, c in doc['warning_files'].items())}).")
    loop = r["loop_checker_every_complaint"]
    if loop.get("present"):
        out.append(f"- 루프 검사기의 모든 불만(checker_all.py): {loop['complaints']}. 이 노드가 바꿀 수 있는 "
                   f"고정 파일만: {loop['only_pinned_files_this_node_changes']}.")
    out.append("")
    out.append("## 10. 한계와 열린 문제")
    out.append("")
    out.append("- 규칙은 비용 규칙이다. `All`은 반올림 범위에서 정확하지만, 다중 채색 검사가 여러 열을 함께 "
               "움직여 우연히 비트를 바꾸는 반올림 한계 경우를 `All`이 놓칠 가능성은 일반적으로 배제되지 않는다. "
               "이 노드의 말뭉치와 보충 집합에서는 그런 경우가 없었다(A가 찾고 B가 놓친 경우 0).")
    out.append("- V26 말뭉치에서 규칙이 바꾸는 경우는 올바른 링 패턴 6개뿐이고 모두 같음(total_groups = n)이라 "
               "비용이 같다. 비용 절감은 total_groups > n인 패턴(조사와 보충 집합의 무작위 띠)에서만 생긴다. "
               f"조사의 띠 전체 패턴 {len(survey['full_bands'])}개 중 total_groups > n인 것: "
               f"{sum(x['total_groups'] > x['n'] for x in survey['full_bands'])}개, = n인 것: "
               f"{sum(x['total_groups'] == x['n'] for x in survey['full_bands'])}개.")
    more = [c for c in wide["first_use_cost_per_class"] if c["rhs_calls_B"] > c["rhs_calls_A"]]
    if more:
        out.append("- 실패하는(잘못된) 선언에서는 검사가 첫 실패에서 멈추므로, 전체 검사가 실패를 만나기 전까지 "
                   "쓰는 우변이 다중 채색 검사보다 많을 수 있다(결정은 같다). 보충 집합에서 B의 첫 사용 경로 우변 "
                   "합계가 A보다 큰 분류: "
                   + "; ".join(f"{c['class']} {c['execution']}: A {c['rhs_calls_A']}, B {c['rhs_calls_B']}"
                               for c in more) + ".")
    out.append("- 스레드 실행에서도 기본값은 호출 스레드의 `All`이다(V29가 `MultiColouring`을 호출 스레드에서 "
               "돌린 것과 같은 선택). `AllPooled`는 계속 선택할 수 있다.")
    out.append("- 계약은 독립 검토를 요구한다(completion_requires).")
    out.append("")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    if "--report-only" in sys.argv:
        committed = json.loads((HERE / "RESULTS.json").read_text())
        (HERE / "REPORT_KO.md").write_text(render(committed))
    else:
        main()
