#!/usr/bin/env python3
"""Parser of loop node V29 (default first-use check of the assembly client, OD-L1-24).

Reads only raw logs: logs/ of this directory (recordings A and B, unit.log, checks.log), the identity
logs of node V08 (jacobian_assembly/logs/identity_<cell>.log), the corpus logs of node V26
(assembly_check/logs/corpus_B_<group>.log) and the review-probe logs of ledger row L-0338
(assembly_check/review_probe/logs_rerun/). It also reads the git diff of the test files against the
base commit to list the edited test lines. Writes RESULTS.json (strict JSON) and REPORT_KO.md.

    python3 -I research/production_loop_20261116/assembly_default/parse_results.py
"""
import json
import re
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
LOGS = HERE / "logs"
V08_LOGS = HERE.parent / "jacobian_assembly" / "logs"
V26_LOGS = HERE.parent / "assembly_check" / "logs"
PROBE_LOGS = HERE.parent / "assembly_check" / "review_probe" / "logs_rerun"
BASE = "e36fc6f405d619ff2ae5b71581252d8995c1ee92"
CELLS = [
    "nlkpp-dense-n400",
    "bruss1d-n128-dense",
    "nlkpp-band-n4096-r24",
    "bruss1d-n131072",
    "bruss1d-n128-coloured",
]
GROUPS = ["controls", "ring", "omit128", "omit1024", "omit131072", "narrow", "correct"]
CLIENT_ARMS = ["seq", "t1", "t2", "t4", "t6"]
WRITE_ALLOWLIST = [
    "crates/rodas5p-integrators/src/stage_clients/assembly.rs",
    "crates/rodas5p-integrators/src/stage_clients/assembly/",
    "crates/rodas5p-integrators/tests/stage_clients_assembly_",
    "research/production_loop_20261116/assembly_default/",
]
# Every line of an existing test file that this node edits, with its reason (the task allows only
# lines that pin the default check or the counts of the default first-use check).
EDIT_REASONS = {
    ("crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs", 608): (
        "comment that called Sample(8) the default first-use check; the block configures Sample(8) "
        "explicitly through spec() (unchanged assertions (1, 8)), and Sample(8) is no longer the "
        "default of the coloured route"
    ),
}
EDIT_REASONS_KO = {
    ("crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs", 608): (
        "Sample(8)을 기본 첫 사용 검사라고 부르던 주석. 이 블록은 spec()으로 Sample(8)을 명시적으로 지정하고 "
        "단언 (1, 8)은 그대로다. Sample(8)은 더 이상 채색 경로의 기본값이 아니다"
    ),
}


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


# ---------------------------------------------------------------------------------------------
# Identity cells
# ---------------------------------------------------------------------------------------------

INVARIANT_TOP = [
    "outcome", "success", "stop_reason", "attempts", "accepted_steps", "rejected_steps",
    "jacobian_reuses", "lu", "driver", "t_end_bits", "final_fingerprint", "output_fingerprint",
    "outputs", "colours", "rhs_per_build", "workers",
]
INVARIANT_STATS = [
    "builds", "failed_builds", "assembly_rhs_calls", "failed_build_rhs_calls", "state_fingerprint",
    "jacobian_fingerprint",
]
CHECK_COUNTERS = ["verifications", "verified_columns", "verification_rhs_calls"]
PROTECTED_FIELDS = [
    "success", "stop_reason", "attempts", "accepted_steps", "rejected_steps", "jacobian_reuses", "lu",
    "driver", "t_end_bits", "final_fingerprint", "output_fingerprint", "outputs", "builds",
    "state_fingerprint", "jacobian_fingerprint", "counters",
]


def identity_records(label):
    """(cell, backend, arm) -> record, cell records, headers."""
    runs, cells, headers = {}, {}, {}
    for cell in CELLS:
        header, records, exits = read_log(LOGS / f"identity_{label}_{cell}.log")
        headers[cell] = {**header, "exit_codes": exits}
        for record in tagged(records, "V29_CELL_JSON"):
            cells[(record["cell"], record["backend"])] = record
        for record in tagged(records, "V29_IDENTITY_JSON"):
            key = (record["cell"], record["backend"], record["arm"])
            if key in runs:
                raise ValueError(f"duplicate identity record {key} in {label}")
            runs[key] = record
    return runs, cells, headers


def v08_identity():
    """(cell, arm) -> the V08 identity records (two per arm for the cell with two back ends)."""
    runs = {}
    for cell in CELLS:
        _, records, _ = read_log(V08_LOGS / f"identity_{cell}.log")
        for record in tagged(records, "V08_IDENTITY_JSON"):
            runs.setdefault((record["cell"], record["arm"]), []).append(record)
    return runs


def compare_identity(a_runs, b_runs, cells_b):
    rows, mismatches = [], []
    for key in sorted(a_runs, key=lambda k: (CELLS.index(k[0]), k[1], k[2])):
        a, b = a_runs[key], b_runs.get(key)
        cell, backend, arm = key
        if b is None:
            mismatches.append({"key": list(key), "field": "missing in B"})
            continue
        if arm == "prot":
            for field in PROTECTED_FIELDS:
                if a.get(field) != b.get(field):
                    mismatches.append({"key": list(key), "field": field, "A": a.get(field), "B": b.get(field)})
            rows.append({
                "cell": cell, "backend": backend, "arm": arm, "bits_equal": True,
                "jacobian_fingerprint": a["jacobian_fingerprint"], "state_fingerprint": a["state_fingerprint"],
                "final_fingerprint": a["final_fingerprint"], "t_end_bits": a["t_end_bits"],
            })
            continue
        before = len(mismatches)
        for field in INVARIANT_TOP:
            if a.get(field) != b.get(field):
                mismatches.append({"key": list(key), "field": field, "A": a.get(field), "B": b.get(field)})
        for field in INVARIANT_STATS:
            if a["stats"].get(field) != b["stats"].get(field):
                mismatches.append({"key": list(key), "field": "stats." + field,
                                   "A": a["stats"].get(field), "B": b["stats"].get(field)})
        delta = b["stats"]["verification_rhs_calls"] - a["stats"]["verification_rhs_calls"]
        for field, value in a["counters"].items():
            expected = value + delta if field in ("rhs_calls", "rhs_evaluations") else value
            if b["counters"].get(field) != expected:
                mismatches.append({"key": list(key), "field": "counters." + field, "A": value,
                                   "B": b["counters"].get(field), "expected_B": expected})
        callback_expected = a["rhs_calls_counted_by_the_callback"] + delta
        if b["rhs_calls_counted_by_the_callback"] != callback_expected:
            mismatches.append({"key": list(key), "field": "rhs_calls_counted_by_the_callback",
                               "A": a["rhs_calls_counted_by_the_callback"],
                               "B": b["rhs_calls_counted_by_the_callback"]})
        # The documented amount of the first-use check.
        coloured = a["route"] == "coloured"
        threaded = arm != "seq"
        if coloured:
            total = cells_b[(cell, backend)]["check_colourings"]["total_groups"]
            expected_a = {"first_use_check": "Sample(8)", "verifications": 1, "verified_columns": 8,
                          "verification_rhs_calls": 8 + threaded}
            expected_b = {"first_use_check": "MultiColouring", "verifications": 1,
                          "verified_columns": a["n"], "verification_rhs_calls": total + threaded}
        else:
            total = None
            expected_a = {"first_use_check": "Sample(8)", "verifications": int(threaded),
                          "verified_columns": 8 * threaded, "verification_rhs_calls": 9 * threaded}
            expected_b = dict(expected_a)
        observed_a = {"first_use_check": a["first_use_check"],
                      **{field: a["stats"][field] for field in CHECK_COUNTERS}}
        observed_b = {"first_use_check": b["first_use_check"],
                      **{field: b["stats"][field] for field in CHECK_COUNTERS}}
        if observed_a != expected_a:
            mismatches.append({"key": list(key), "field": "first-use check of A", "observed": observed_a,
                               "expected": expected_a})
        if observed_b != expected_b:
            mismatches.append({"key": list(key), "field": "first-use check of B", "observed": observed_b,
                               "expected": expected_b})
        rows.append({
            "cell": cell, "backend": backend, "arm": arm, "route": a["route"], "n": a["n"],
            "bits_equal": len(mismatches) == before,
            "jacobian_fingerprint": a["stats"]["jacobian_fingerprint"],
            "state_fingerprint": a["stats"]["state_fingerprint"],
            "final_fingerprint": a["final_fingerprint"], "output_fingerprint": a["output_fingerprint"],
            "t_end_bits": a["t_end_bits"], "attempts": a["attempts"], "accepted_steps": a["accepted_steps"],
            "builds": a["stats"]["builds"],
            "first_use_check": {"A": a["first_use_check"], "B": b["first_use_check"]},
            "verifications": {"A": a["stats"]["verifications"], "B": b["stats"]["verifications"]},
            "verified_columns": {"A": a["stats"]["verified_columns"], "B": b["stats"]["verified_columns"]},
            "verification_rhs_calls": {"A": a["stats"]["verification_rhs_calls"],
                                       "B": b["stats"]["verification_rhs_calls"]},
            "rhs_calls": {"A": a["counters"]["rhs_calls"], "B": b["counters"]["rhs_calls"]},
            "rhs_calls_counted_by_the_callback": {"A": a["rhs_calls_counted_by_the_callback"],
                                                  "B": b["rhs_calls_counted_by_the_callback"]},
            "multi_colouring_total_groups": total,
            "documented_delta_rhs_calls": delta,
        })
    extra = sorted(set(b_runs) - set(a_runs))
    for key in extra:
        mismatches.append({"key": list(key), "field": "missing in A"})
    return rows, mismatches


def compare_with_v08(a_runs, v08):
    """Commit A against the V08 identity logs (the same Sample(8) check, today's code)."""
    compared, mismatches = 0, []
    for (cell, backend, arm), a in a_runs.items():
        candidates = v08.get((cell, arm), [])
        if arm == "prot":
            reference = [c for c in candidates
                         if c["jacobian_fingerprint"] == a["jacobian_fingerprint"]]
            fields = {"attempts": a["attempts"], "accepted_steps": a["accepted_steps"],
                      "t_end_bits": a["t_end_bits"], "final_fingerprint": a["final_fingerprint"],
                      "state_fingerprint": a["state_fingerprint"], "builds": a["builds"],
                      "rhs_calls_without_jacobian": a["counters"]["rhs_calls"]}
        else:
            reference = [c for c in candidates if c["backend"] == backend]
            fields = {"attempts": a["attempts"], "accepted_steps": a["accepted_steps"],
                      "rejected_steps": a["rejected_steps"], "t_end_bits": a["t_end_bits"],
                      "final_fingerprint": a["final_fingerprint"],
                      "jacobian_fingerprint": a["stats"]["jacobian_fingerprint"],
                      "state_fingerprint": a["stats"]["state_fingerprint"],
                      "rhs_calls": a["counters"]["rhs_calls"],
                      "assembly_rhs_calls": a["stats"]["assembly_rhs_calls"],
                      "verification_rhs_calls": a["stats"]["verification_rhs_calls"],
                      "verified_columns": a["stats"]["verified_columns"],
                      "verifications": a["stats"]["verifications"], "builds": a["stats"]["builds"],
                      "colours": a["colours"], "rhs_per_build": a["rhs_per_build"],
                      "rhs_calls_counted_by_the_callback": a["rhs_calls_counted_by_the_callback"],
                      "jacobian_builds": a["counters"]["jacobian_builds"],
                      "direct_factorizations": a["counters"]["direct_factorizations"],
                      "direct_solve_calls": a["counters"]["direct_solve_calls"]}
        if not reference:
            mismatches.append({"key": [cell, backend, arm], "field": "no V08 record"})
            continue
        v08_record = reference[0]
        for field, value in fields.items():
            if v08_record.get(field) != value:
                mismatches.append({"key": [cell, backend, arm], "field": field, "A": value,
                                   "V08": v08_record.get(field)})
        compared += 1
    return compared, mismatches


# ---------------------------------------------------------------------------------------------
# Corpus
# ---------------------------------------------------------------------------------------------

CASE_FIELDS = ["problem", "n", "route", "backend", "declared_nnz", "client_colours", "pure", "correct",
               "omitted_entries", "omitted_first"]


def decision_key(record):
    if record["outcome"] == "pass":
        return ("pass", record.get("columns"))
    return (record["outcome"], record.get("kind"), record.get("row"), record.get("column"),
            record.get("colour"), record.get("colouring"), record.get("typed"))


def detected(record):
    return record["outcome"] == "error" and record.get("contract_violation") is True


def corpus_records(label):
    cases, headers, groups = {}, {}, {}
    for group in GROUPS:
        header, records, exits = read_log(LOGS / f"corpus_{label}_{group}.log")
        headers[group] = {**header, "exit_codes": exits}
        for record in tagged(records, "V29_CASE_JSON"):
            if record["case"] in cases:
                raise ValueError("duplicate case " + record["case"])
            cases[record["case"]] = record
        for record in tagged(records, "V29_GROUP_JSON"):
            groups[record["group"]] = record
    return cases, headers, groups


def v26_corpus():
    cases = {}
    for group in GROUPS:
        _, records, _ = read_log(V26_LOGS / f"corpus_B_{group}.log")
        for record in tagged(records, "V26_CASE_JSON"):
            cases[record["case"]] = record
    return cases


def reference_arm(route, execution, label):
    """The V26 arm a decision of the default is compared with (None: no V26 arm)."""
    if label == "A" or route == "dense":
        return {"seq": "S8@seq", "t3": "S8@t3"}.get(execution)
    return {"seq": "MC@seq", "t6": "MC@t6", "t3": "MC@t6"}.get(execution)


def analyse_corpus(a_cases, b_cases, v26):
    result = {"case_set": {}, "per_group": [], "mismatches": []}
    names = sorted(v26)
    same_set = sorted(a_cases) == names and sorted(b_cases) == names
    field_mismatches = []
    for name in names:
        for label, cases in (("A", a_cases), ("B", b_cases)):
            for field in CASE_FIELDS:
                if cases.get(name, {}).get(field) != v26[name].get(field):
                    field_mismatches.append({"case": name, "recording": label, "field": field})
    result["case_set"] = {
        "cases_v26": len(names), "cases_A": len(a_cases), "cases_B": len(b_cases),
        "same_case_ids": same_set, "field_mismatches": field_mismatches[:50],
        "field_mismatch_count": len(field_mismatches),
    }
    compared = {"A": 0, "B": 0}
    path_mismatches, reference_mismatches, ab_dense_mismatches = [], [], []
    default_checks = {"A": {}, "B": {}}
    detect = {"A": set(), "B": set()}
    misses = {"A": [], "B": []}
    first_use_misses = {"A": [], "B": []}
    rejected_correct = []
    v26_mc_misses = set()
    group_rows = {group: {"group": group, "cases": 0, "wrong": 0, "A_misses_seq": 0, "B_misses_seq": 0,
                          "B_misses_t6": 0, "v26_MC_misses_seq": 0, "v26_S8_misses_seq": 0,
                          "B_equal_to_v26_reference": 0, "B_compared_with_v26": 0,
                          "A_equal_to_v26_reference": 0, "A_compared_with_v26": 0}
                  for group in GROUPS}
    for name in names:
        reference = v26[name]
        group = reference["group"]
        row = group_rows[group]
        row["cases"] += 1
        wrong = not reference["correct"]
        row["wrong"] += int(wrong)
        if wrong and reference["arms"]["MC@seq"]["outcome"] == "pass":
            v26_mc_misses.add(name)
            row["v26_MC_misses_seq"] += 1
        if wrong and reference["arms"]["S8@seq"]["outcome"] == "pass":
            row["v26_S8_misses_seq"] += 1
        for label, cases in (("A", a_cases), ("B", b_cases)):
            record = cases[name]
            for execution, decisions in record["executions"].items():
                check = decisions["first_use_check"]
                default_checks[label].setdefault((record["route"], execution), set()).add(check)
                first, checked = decisions["first_use"], decisions["verify"]
                sequential_dense = record["route"] == "dense" and execution == "seq"
                if sequential_dense:
                    # The sequential dense route makes no first-use check.
                    if not (first["outcome"] == "pass" and first["verifications"] == 0):
                        path_mismatches.append({"case": name, "recording": label, "execution": execution,
                                                "first_use": first})
                elif decision_key(first) != decision_key(checked):
                    path_mismatches.append({"case": name, "recording": label, "execution": execution,
                                            "first_use": decision_key(first), "verify": decision_key(checked)})
                arm = reference_arm(record["route"], execution, label)
                if arm is not None and arm in reference["arms"]:
                    compared[label] += 1
                    row[f"{label}_compared_with_v26"] += 1
                    if decision_key(checked) == decision_key(reference["arms"][arm]):
                        row[f"{label}_equal_to_v26_reference"] += 1
                    else:
                        reference_mismatches.append({
                            "case": name, "recording": label, "execution": execution, "v26_arm": arm,
                            "default": decision_key(checked), "v26": decision_key(reference["arms"][arm])})
                if wrong:
                    if detected(checked):
                        detect[label].add((name, execution))
                    else:
                        misses[label].append([name, execution])
                        if execution == "seq":
                            row[f"{label}_misses_seq"] += 1
                        elif execution == "t6" and label == "B":
                            row["B_misses_t6"] += 1
                    if not detected(first):
                        first_use_misses[label].append([name, execution])
                elif checked["outcome"] != "pass" or first["outcome"] != "pass":
                    rejected_correct.append([name, label, execution])
        # The dense route keeps its check: A and B decide alike there in every execution.
        if reference["route"] == "dense":
            for execution in a_cases[name]["executions"]:
                a_dec = a_cases[name]["executions"][execution]
                b_dec = b_cases[name]["executions"][execution]
                if decision_key(a_dec["verify"]) != decision_key(b_dec["verify"]) or \
                        decision_key(a_dec["first_use"]) != decision_key(b_dec["first_use"]):
                    ab_dense_mismatches.append([name, execution])
    regressions = sorted(detect["A"] - detect["B"])
    a_miss_set = {tuple(miss) for miss in misses["A"]}
    b_miss_set = {tuple(miss) for miss in misses["B"]}
    evidence = []
    for name, execution in sorted(b_miss_set):
        record = b_cases[name]
        proof = record["miss_evidence"].get(execution)
        evidence.append({
            "case": name, "execution": execution,
            "rounding_limited": bool(proof and proof["rounding_limited"]),
            "states": [state["state"] for entry in (proof or {}).get("entries", []) for state in entry["states"]],
            "v26_MC_seq": decision_key(v26[name]["arms"]["MC@seq"]),
            "v26_FA_seq": decision_key(v26[name]["arms"]["FA@seq"]),
        })
    result["per_group"] = [group_rows[group] for group in GROUPS]
    result["decisions_compared_with_v26"] = compared
    result["decision_mismatches_with_v26"] = reference_mismatches
    result["first_use_and_verify_disagree"] = path_mismatches
    result["dense_route_A_B_disagree"] = ab_dense_mismatches
    result["default_checks"] = {label: {f"{route}@{execution}": sorted(checks)
                                        for (route, execution), checks in sorted(table.items())}
                                for label, table in default_checks.items()}
    result["misses_A"] = {"count": len(misses["A"]), "seq": sum(1 for _, e in misses["A"] if e == "seq"),
                          "t6": sum(1 for _, e in misses["A"] if e == "t6")}
    result["misses_B"] = {"count": len(misses["B"]), "list": sorted(misses["B"])}
    result["first_use_path_misses_B"] = {
        "note": "wrong cases that pass the first linearization of a default client; the sequential dense "
                "route makes no first-use check (unchanged), so the impure dense control passes there",
        "count": len(first_use_misses["B"]), "list": sorted(first_use_misses["B"]),
        "count_A": len(first_use_misses["A"]),
    }
    result["misses_B_evidence"] = evidence
    result["detected_by_A_not_by_B"] = [list(item) for item in regressions]
    result["A_misses_detected_by_B"] = len(a_miss_set - b_miss_set)
    result["v26_MC_misses_seq"] = sorted(v26_mc_misses)
    result["correct_rejected"] = rejected_correct
    return result


# ---------------------------------------------------------------------------------------------
# Unit tests, checks, review probe, edited lines
# ---------------------------------------------------------------------------------------------

def unit_log():
    path = LOGS / "unit.log"
    if not path.exists():
        return None
    text = path.read_text(encoding="utf-8", errors="replace")
    header, records, exits = read_log(path)
    tests = re.findall(r"^test (\S+) \.\.\. (\w+)", text, re.MULTILINE)
    results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
    return {
        "git_head": header.get("git head"), "tracked_changes": header.get("git status (tracked changes)"),
        "exit_codes": exits,
        "tests": [{"name": name, "result": outcome} for name, outcome in tests],
        "passed": sum(int(r[1]) for r in results), "failed": sum(int(r[2]) for r in results),
        "allocation": tagged(records, "STAGE_CLIENTS_ASSEMBLY_DEFAULT_ALLOC_JSON"),
    }


def checks_log():
    path = LOGS / "checks.log"
    if not path.exists():
        return None
    header, records, _ = read_log(path)
    return {"git_head": header.get("git head"), "tracked_changes": header.get("git status (tracked changes)"),
            "checks": tagged(records, "V29_CHECK_JSON")}


def checker_all_log(changed_paths):
    """Every complaint of the loop checker (checker_all.py) and whether each one only names a pinned
    implementation file that this node changed inside its write allowlist."""
    path = LOGS / "checker_all.log"
    if not path.exists():
        return None
    header, records, exits = read_log(path)
    found = tagged(records, "V29_CHECKER_JSON")
    complaints = found[0]["complaints"] if found else []
    prefix = "implementation changed; rerun affected tests/update checkpoint: "
    expected = [complaint for complaint in complaints
                if complaint.startswith(prefix)
                and complaint[len(prefix):] in changed_paths
                and any(complaint[len(prefix):].startswith(allowed) for allowed in WRITE_ALLOWLIST)]
    return {
        "git_head": header.get("git head"), "tracked_changes": header.get("git status (tracked changes)"),
        "exit_codes": exits, "complaints": complaints,
        "only_pinned_files_this_node_changes": bool(found) and len(expected) == len(complaints),
    }


def probe_limits():
    rows = []
    for name in ("probe_cancel.log", "probe_collapse.log"):
        path = PROBE_LOGS / name
        if not path.exists():
            continue
        _, records, _ = read_log(path)
        for record in tagged(records, "REVIEW_JSON"):
            if "decisions" not in record:
                continue
            decisions = record["decisions"]
            rows.append({
                "probe": record["probe"],
                "case": record.get("case") or f"collapse trial {record.get('trial')}",
                "S8@seq": decisions["S8@seq"]["detected"],
                "MC@seq": decisions["MC@seq"]["detected"],
                "FA@seq": decisions["FA@seq"]["detected"],
                "FA_P@t6": decisions["FA_P@t6"]["detected"],
            })
    return rows


def edited_lines():
    changed = git("diff", "--name-status", BASE, "HEAD").splitlines()
    paths = [line.split("\t") for line in changed if line]
    outside = [path for _, path in paths if not any(path.startswith(prefix) for prefix in WRITE_ALLOWLIST)]
    existing_tests, new_tests = [], []
    for status, path in paths:
        if path.startswith("crates/rodas5p-integrators/tests/"):
            (new_tests if status == "A" else existing_tests).append(path)
    edits = []
    for path in existing_tests:
        diff = git("diff", "--unified=0", BASE, "HEAD", "--", path)
        old_line = new_line = 0
        for line in diff.splitlines():
            hunk = re.match(r"^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@", line)
            if hunk:
                old_line, new_line = int(hunk.group(1)), int(hunk.group(2))
                continue
            if line.startswith("---") or line.startswith("+++"):
                continue
            if line.startswith("-"):
                edits.append({"file": path, "line_at_base": old_line, "removed": line[1:]})
                old_line += 1
            elif line.startswith("+"):
                edits.append({"file": path, "line": new_line, "added": line[1:]})
                new_line += 1
    by_line = {}
    for edit in edits:
        line = edit.get("line") or edit.get("line_at_base")
        entry = by_line.setdefault((edit["file"], line), {"file": edit["file"], "line": line})
        entry.update({key: value for key, value in edit.items() if key in ("removed", "added")})
    listed = []
    for (path, line), entry in sorted(by_line.items()):
        entry["reason"] = EDIT_REASONS.get((path, line))
        listed.append(entry)
    new_test_names = {}
    for path in new_tests:
        text = (ROOT / path).read_text()
        new_test_names[path] = re.findall(r"#\[test\]\s*(?:#\[ignore[^\]]*\]\s*)?fn (\w+)", text)
    return {
        "changed_paths_since_base": [path for _, path in paths],
        "paths_outside_the_write_allowlist": outside,
        "edited_lines_of_existing_tests": listed,
        "edited_lines_without_reason": [entry for entry in listed if not entry["reason"]],
        "new_test_files": new_test_names,
    }


# ---------------------------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------------------------

def main():
    a_runs, a_cells, a_headers = identity_records("A")
    b_runs, b_cells, b_headers = identity_records("B")
    identity_rows, identity_mismatches = compare_identity(a_runs, b_runs, b_cells)
    v08_compared, v08_mismatches = compare_with_v08(a_runs, v08_identity())
    a_cases, a_corpus_headers, a_groups = corpus_records("A")
    b_cases, b_corpus_headers, b_groups = corpus_records("B")
    corpus = analyse_corpus(a_cases, b_cases, v26_corpus())
    unit = unit_log()
    checks = checks_log()
    edits = edited_lines()
    limits = probe_limits()
    checker_all = checker_all_log(edits["changed_paths_since_base"])

    def provenance(headers):
        keys = ["git head", "git status (tracked changes)", "client sources sha256", "program source sha256",
                "binary sha256"]
        values = {key: sorted({header.get(key) for header in headers.values()}) for key in keys}
        values["exit_codes"] = sorted({code for header in headers.values() for code in header["exit_codes"]})
        return values

    recordings = {
        "A": {"identity": provenance(a_headers), "corpus": provenance(a_corpus_headers)},
        "B": {"identity": provenance(b_headers), "corpus": provenance(b_corpus_headers)},
    }
    same_program = (recordings["A"]["identity"]["program source sha256"]
                    == recordings["B"]["identity"]["program source sha256"]
                    == recordings["A"]["corpus"]["program source sha256"]
                    == recordings["B"]["corpus"]["program source sha256"])
    clean = all(value == ["0"] for label in recordings for part in recordings[label].values()
                for key, value in part.items() if key == "git status (tracked changes)")
    exits_ok = all(part["exit_codes"] == [0] for label in recordings for part in recordings[label].values())

    # Default per route.
    default_cells = {}
    for label, cells in (("A", a_cells), ("B", b_cells)):
        for (cell, backend), record in cells.items():
            checks_of_cell = sorted({entry["first_use_check"] for entry in record["default_checks"]})
            default_cells.setdefault(f"{cell}/{backend}", {"route": record["route"]})[label] = checks_of_cell
    per_route_ok = all(
        entry["A"] == ["Sample(8)"]
        and entry["B"] == (["MultiColouring"] if entry["route"] == "coloured" else ["Sample(8)"])
        for entry in default_cells.values())
    corpus_defaults = corpus["default_checks"]
    corpus_defaults_ok = (
        all(value == ["Sample(8)"] for value in corpus_defaults["A"].values())
        and all(value == (["MultiColouring"] if key.startswith("coloured") else ["Sample(8)"])
                for key, value in corpus_defaults["B"].items()))
    unit_names = {test["name"]: test["result"] for test in (unit or {}).get("tests", [])}
    per_route_tests = ["the_default_first_use_check_follows_the_route",
                       "the_default_check_runs_once_at_the_first_linearization_on_every_route"]
    per_route_tests_ok = all(unit_names.get(name) == "ok" for name in per_route_tests)

    # Suites.
    check_rows = (checks or {}).get("checks", [])
    by_name = {row["name"]: row for row in check_rows}

    def check_ok(name):
        row = by_name.get(name)
        return bool(row) and row["exit_code"] == 0 and row.get("failed", 0) == 0

    required_checks = ("stage client tests", "sentinels", "clippy", "fmt")
    suites_ok = all(check_ok(name) for name in required_checks)
    suites_complete = all(name in by_name for name in required_checks)
    loop_checker = by_name.get("loop checker")

    bits_ok = not identity_mismatches and len(identity_rows) == len(a_runs) == len(b_runs)
    corpus_ok = (
        corpus["case_set"]["same_case_ids"] and corpus["case_set"]["field_mismatch_count"] == 0
        and not corpus["decision_mismatches_with_v26"] and not corpus["first_use_and_verify_disagree"]
        and not corpus["dense_route_A_B_disagree"] and not corpus["detected_by_A_not_by_B"]
        and not corpus["correct_rejected"]
        and all(item["rounding_limited"] for item in corpus["misses_B_evidence"])
        and sorted({name for name, _ in corpus["misses_B"]["list"]}) == corpus["v26_MC_misses_seq"]
        and unit_names.get("the_default_decides_as_the_multi_colouring_check_on_part_of_the_v26_corpus") == "ok"
    )
    coloured_rows = [row for row in identity_rows if row.get("route") == "coloured"]
    dense_rows = [row for row in identity_rows if row.get("route") == "dense"]
    discriminators = [
        {
            "name": "bits unchanged",
            "criterion": "TASK_CONTRACTS V29 acceptance 1: the Jacobian and trajectory bits of the V08 identity "
                         "cells equal commit A; only the counters of the first-use check change, by the "
                         "documented amount",
            "measured": f"{len(identity_rows)} runs (5 cells, both band back ends of nlkpp-band-n4096-r24, arms "
                        f"prot, seq, t1, t2, t4, t6): {sum(row['bits_equal'] for row in identity_rows)} equal to "
                        f"commit A in Jacobian, linearization-point, output and final-state fingerprints, steps "
                        f"and every counter but the check's; {len(identity_mismatches)} mismatches; coloured "
                        f"cells change only verification RHS (8 -> total_groups, +1 threaded), verified columns "
                        f"(8 -> n) and rhs_calls by the same amount; dense cells unchanged; commit A equals the "
                        f"V08 identity logs in {v08_compared} runs ({len(v08_mismatches)} mismatches)",
            "verdict": "PASS" if bits_ok and not v08_mismatches and unit_names.get(
                "a_run_with_the_default_check_keeps_every_bit") == "ok" else "FAIL",
        },
        {
            "name": "default per route",
            "criterion": "TASK_CONTRACTS V29 acceptance 3 (new tests: the default per route) and OD-L1-24",
            "measured": "identity cells: " + "; ".join(
                f"{key} {entry['A']} -> {entry['B']}" for key, entry in sorted(default_cells.items()))
                        + f"; corpus: {json.dumps(corpus_defaults['B'], sort_keys=True)}; tests "
                        + ", ".join(f"{name} {unit_names.get(name)}" for name in per_route_tests),
            "verdict": "PASS" if per_route_ok and corpus_defaults_ok and per_route_tests_ok else "FAIL",
        },
        {
            "name": "corpus detection of the default",
            "criterion": "TASK_CONTRACTS V29 acceptance 3: the default detects the corpus cases that MC detects",
            "measured": f"{corpus['case_set']['cases_v26']} V26 cases, same ids and fields: "
                        f"{corpus['case_set']['same_case_ids'] and corpus['case_set']['field_mismatch_count'] == 0}; "
                        f"decisions of B equal to the V26 reference arm in "
                        f"{sum(row['B_equal_to_v26_reference'] for row in corpus['per_group'])} of "
                        f"{corpus['decisions_compared_with_v26']['B']} (MC@seq/MC@t6 coloured, S8@seq/S8@t3 "
                        f"dense); misses of B {corpus['misses_B']['count']} (case, execution) = the V26 MC miss "
                        f"{corpus['v26_MC_misses_seq']}, rounding-limited; misses of A (Sample(8)) "
                        f"{corpus['misses_A']['count']} ({corpus['misses_A']['seq']} seq), of which "
                        f"{corpus['A_misses_detected_by_B']} detected by B; detected by A and not by B: "
                        f"{len(corpus['detected_by_A_not_by_B'])}; correct patterns rejected: "
                        f"{len(corpus['correct_rejected'])}",
            "verdict": "PASS" if corpus_ok else "FAIL",
        },
        {
            "name": "suites",
            "criterion": "TASK_CONTRACTS V29 acceptance 2 and 4: existing tests pass (edits only with a listed "
                         "reason), clippy -D warnings and fmt",
            "measured": "; ".join(
                f"{row['name']}: exit {row['exit_code']}"
                + (f", {row['passed']} passed / {row['failed']} failed / {row['ignored']} ignored in "
                   f"{row['binaries']} binaries" if "passed" in row else "")
                for row in check_rows) + f"; edited lines without reason: "
                                         f"{len(edits['edited_lines_without_reason'])}",
            "verdict": "PASS" if suites_ok and not edits["edited_lines_without_reason"]
            and not edits["paths_outside_the_write_allowlist"] else (
                "NOT_RUN" if checks is None or not suites_complete else "FAIL"),
        },
    ]
    results = {
        "schema": "vigilode.loop-l1.v29-results.v1",
        "node": "V29_ASSEMBLY_DEFAULT_MC",
        "authority": "owner decision OD-L1-24 (2026-11-24 23:45 KST, '권고사항 수용 및 CI job 유지'): the default "
                     "first-use check of the opt-in AssemblyStageClient becomes the multi-colouring check MC on "
                     "the routes where it applies (coloured, banded); the dense route keeps Sample(8); FA, FA_P "
                     "and MC_P stay selectable; the documentation states the limits of MC (L-0338)",
        "contract": "docs/agent/production_solver_program_20261112/TASK_CONTRACTS.json, id V29_ASSEMBLY_DEFAULT_MC",
        "base_commit": BASE,
        "head_at_parse": git("rev-parse", "HEAD").strip(),
        "protocol": {
            "note": "Implementation of an owner decision, no experiment and no numerical threshold; the "
                    "discriminators are the equalities of the contract's acceptance checks. The protocol choices "
                    "(cells, arms, corpus, executions, the two decision paths, the V26 reference arms) were "
                    "committed before the first run in NOTES.txt entry 1 (ec284167, 2026-11-24 23:56 KST) and "
                    "in the recording program (f1eb5118, 2026-11-25 00:03 KST); first run 00:03. Deviation of "
                    "form: no file named PROTOCOL.json was written.",
            "reference_arms": "A (Sample(8) everywhere): S8@seq, S8@t3 of V26; B coloured: MC@seq for seq, MC@t6 "
                              "for t6 and t3; B dense: S8@seq, S8@t3; t6 on the dense route: A against B",
            "decision_paths": "first_use = first linearize of a fresh default client; verify = "
                              "AssemblyStageClient::verify(t0, y0, options.first_use_check) on a fresh default "
                              "client (the V26 convention)",
        },
        "recordings": recordings,
        "recording_checks": {"same_program_source": same_program, "clean_trees": clean, "exit_codes_zero": exits_ok},
        "default_per_route": {"identity_cells": default_cells, "corpus": corpus_defaults},
        "identity": {
            "runs": identity_rows,
            "mismatches_A_B": identity_mismatches,
            "commit_A_against_v08": {"runs_compared": v08_compared, "mismatches": v08_mismatches},
        },
        "corpus": corpus,
        "limits_of_mc_l0338_from_the_review_probe_logs": limits,
        "unit_tests_of_this_node": unit,
        "tests": edits,
        "checks": checks,
        "loop_checker": loop_checker,
        "loop_checker_every_complaint": checker_all,
        "discriminators": discriminators,
    }
    text = json.dumps(results, indent=1, sort_keys=False, ensure_ascii=False, allow_nan=False) + "\n"
    (HERE / "RESULTS.json").write_text(text, encoding="utf-8")
    (HERE / "REPORT_KO.md").write_text(render(results), encoding="utf-8")
    for item in discriminators:
        print(f"{item['name']}: {item['verdict']}")


# ---------------------------------------------------------------------------------------------
# Report (Korean)
# ---------------------------------------------------------------------------------------------

def render(results):
    lines = []
    add = lines.append
    corpus = results["corpus"]
    verdicts = {item["name"]: item["verdict"] for item in results["discriminators"]}
    add("# V29 조립 클라이언트의 기본 첫 사용 검사: 채색 경로는 다중 채색 검사(MC)")
    add("")
    add("이 보고서는 `parse_results.py`가 `logs/`의 원시 로그(기록 A와 B, unit.log, checks.log), V08 동일성 로그, "
        "V26 코퍼스 로그와 L-0338 리뷰 probe 로그만 읽어 `RESULTS.json`과 함께 생성했다. 숫자를 손으로 적지 않았다. "
        f"기준 커밋 `{results['base_commit'][:8]}`, 생성 시점 HEAD `{results['head_at_parse'][:8]}`.")
    add("")
    add("## 1. 결론")
    add("")
    add("- 소유자 결정 OD-L1-24(권고사항 수용): 선택형 `AssemblyStageClient`의 기본 첫 사용 검사는 MC가 적용되는 "
        "경로(채색 경로: 조밀 LU 뒷단과 두 밴드 뒷단, 순차와 스레드 실행 모두)에서 `CheckColumns::MultiColouring`이 "
        "되었다. 조밀 경로는 MC가 적용되지 않으므로(선언된 패턴 없음) `Sample(8)`을 유지한다. FA(`All`), "
        "FA_P(`AllPooled`), MC_P(`MultiColouringPooled`)와 나머지 검사는 그대로 선택할 수 있다.")
    add("- 구현: `AssemblyOptions::default_first_use_check(route)`가 경로별 기본값을 정하고 `dense()`와 "
        "`coloured()`가 이를 쓴다. MC가 첫 사용 검사인 클라이언트는 추가 채색을 `AssemblyStageClient::new`에서 "
        "만든다(첫 선형화에서 할당 없음). 조립 야코비안의 산술은 한 줄도 바뀌지 않았다.")
    add("- 판별 기준: " + ", ".join(f"{name} {verdict}" for name, verdict in verdicts.items()) + ".")
    add("")
    add("| 판별 기준 | 판정 | 측정 |")
    add("|---|---|---|")
    for item in results["discriminators"]:
        add(f"| {item['name']} | {item['verdict']} | {item['measured']} |")
    add("")
    add("## 2. 경로별 기본값")
    add("")
    add("| 셀 / 뒷단 | 경로 | 기본 검사 (변경 전, A) | 기본 검사 (변경 후, B) |")
    add("|---|---|---|---|")
    for key, entry in sorted(results["default_per_route"]["identity_cells"].items()):
        add(f"| {key} | {entry['route']} | {', '.join(entry['A'])} | {', '.join(entry['B'])} |")
    add("")
    add("코퍼스(경로@실행별 기본 검사, B): " + ", ".join(
        f"`{key}` {', '.join(value)}" for key, value in sorted(results["default_per_route"]["corpus"]["B"].items()))
        + ". 순차 조밀 경로는 예전과 같이 첫 사용 검사를 하지 않는다(비교할 두 번째 평가가 없음).")
    add("")
    add("## 3. 비트 불변 (V08 동일성 셀)")
    add("")
    add("같은 기록 프로그램(소스 sha256 `"
        + ", ".join(v[:8] for v in results["recordings"]["A"]["identity"]["program source sha256"])
        + "`)을 변경 전(A)과 변경 후(B)에 실행했다. 모든 클라이언트는 검사를 지정하지 않은 기본 옵션으로 만들었다. "
        "야코비안 지문(모든 조립 야코비안), 선형화 지점 지문, 출력과 최종 상태 지문, 걸음 수, 첫 사용 검사 이외의 "
        "모든 카운터가 A와 같다. 바뀐 것은 첫 사용 검사의 카운터뿐이며, 그 양은 문서화한 양(채색 경로: 검사 RHS "
        "8 → MC 그룹 합, 스레드 실행은 +1; 검사한 열 8 → n; `rhs_calls`도 같은 양)과 정확히 같다.")
    add("")
    add("| 셀 | 뒷단 | 팔 | 비트 같음 | 검사 (A → B) | 검사 RHS (A → B) | 검사 열 (A → B) | rhs_calls (A → B) | 야코비안 지문 |")
    add("|---|---|---|---|---|---|---|---|---|")
    for row in results["identity"]["runs"]:
        if row["arm"] == "prot":
            add(f"| {row['cell']} | {row['backend']} | prot | {'예' if row['bits_equal'] else '아니오'} | - | - | - | - | "
                f"`{row['jacobian_fingerprint']}` |")
            continue
        add(f"| {row['cell']} | {row['backend']} | {row['arm']} | {'예' if row['bits_equal'] else '아니오'} | "
            f"{row['first_use_check']['A']} → {row['first_use_check']['B']} | "
            f"{row['verification_rhs_calls']['A']} → {row['verification_rhs_calls']['B']} | "
            f"{row['verified_columns']['A']} → {row['verified_columns']['B']} | "
            f"{row['rhs_calls']['A']:,} → {row['rhs_calls']['B']:,} | `{row['jacobian_fingerprint']}` |")
    add("")
    v08 = results["identity"]["commit_A_against_v08"]
    add(f"- A와 B의 불일치: {len(results['identity']['mismatches_A_B'])}건. A는 V08 동일성 로그와 "
        f"{v08['runs_compared']}개 실행에서 비교했고 불일치 {len(v08['mismatches'])}건(오늘 코드의 Sample(8) 기본값이 "
        "V08 측정과 같은 비트와 카운터를 냄).")
    add("- 프로그램 단위 테스트 `a_run_with_the_default_check_keeps_every_bit`: 여섯 경로(링 패턴과 밴드의 조밀 LU, "
        "보호 밴드 커널, 타이트 밴드 커널, 조밀 경로 둘), 순차와 2 작업자에서 기본 검사로 돈 전체 실행이 검사 없는 실행과 "
        "모든 출력 비트, 선형화 지점, 야코비안이 같고 카운터는 검사 RHS만큼만 다르다.")
    add("")
    add("## 4. V26 코퍼스에서 기본값의 탐지")
    add("")
    cs = corpus["case_set"]
    add(f"V26 코퍼스 {cs['cases_v26']}건을 이 노드의 기록 프로그램에 옮겨 적었고, 사례 id, 차수, 선언 항목 수, 누락 "
        f"항목, 클라이언트 색 수가 V26 로그와 같다(불일치 {cs['field_mismatch_count']}건). 각 사례의 결정은 새 "
        "클라이언트에서 두 경로로 기록했다: 첫 `linearize`(드라이버 실행이 하는 일)와 `verify(t0, y0, 기본 검사)`"
        "(V26의 결정 규약). 두 경로는 순차 조밀 경로(검사 없음)를 빼고 모든 사례에서 같다"
        f"(불일치 {len(corpus['first_use_and_verify_disagree'])}건).")
    add("")
    add("| 그룹 | 사례 | 잘못된 선언 | A(Sample(8)) 놓침, seq | B(기본) 놓침, seq | B 놓침, t6 | V26 MC 놓침 | V26 S8 놓침 | B = V26 기준 팔 |")
    add("|---|---|---|---|---|---|---|---|---|")
    for row in corpus["per_group"]:
        add(f"| {row['group']} | {row['cases']} | {row['wrong']} | {row['A_misses_seq']} | {row['B_misses_seq']} | "
            f"{row['B_misses_t6']} | {row['v26_MC_misses_seq']} | {row['v26_S8_misses_seq']} | "
            f"{row['B_equal_to_v26_reference']} / {row['B_compared_with_v26']} |")
    add("")
    add(f"- 기준 팔: 채색 경로 B는 V26 MC@seq(seq), MC@t6(t6, t3), 조밀 경로는 S8@seq, S8@t3. B의 결정은 비교한 "
        f"{corpus['decisions_compared_with_v26']['B']}건에서 모두 같다(불일치 {len(corpus['decision_mismatches_with_v26'])}건; "
        "A도 S8 기준과 같음).")
    add(f"- B(새 기본값)가 놓친 경우: {corpus['misses_B']['count']}건(사례, 실행) = "
        + ", ".join(f"`{name}` ({execution})" for name, execution in corpus["misses_B"]["list"])
        + ". V26에서 FA와 MC가 함께 놓친 한 사례이며, B 로그의 증거에서도 MC가 평가한 모든 상태에서 행 비트가 같다"
        "(반올림 한계).")
    add(f"- A(옛 기본값 Sample(8))가 놓친 경우: {corpus['misses_A']['count']}건(seq {corpus['misses_A']['seq']}, "
        f"t6 {corpus['misses_A']['t6']}), 그중 B가 잡은 것 {corpus['A_misses_detected_by_B']}건. A가 잡고 B가 놓친 "
        f"사례: {len(corpus['detected_by_A_not_by_B'])}건. 정답 패턴을 거부한 경우: {len(corpus['correct_rejected'])}건.")
    add("- 조밀 경로 사례는 A와 B의 결정이 모든 실행에서 같다(불일치 "
        f"{len(corpus['dense_route_A_B_disagree'])}건): 조밀 경로의 기본값은 바뀌지 않았다.")
    first_use = corpus["first_use_path_misses_B"]
    add(f"- 첫 선형화 경로로 본 B의 통과(잘못된 선언): {first_use['count']}건 = "
        + ", ".join(f"`{name}` ({execution})" for name, execution in first_use["list"])
        + ". 위 반올림 한계 사례 외에는 순차 조밀 경로의 비순수 대조군뿐이다. 순차 조밀 경로는 비교할 두 번째 평가가 "
        "없어 첫 사용 검사를 하지 않으며(V08 설계, 이 노드에서 바뀌지 않음), A에서도 같다.")
    add("")
    add("## 5. MC가 놓치는 것 (L-0338): 기본값도 그대로 물려받는다")
    add("")
    add("러스트독(`CheckColumns`, `AssemblyOptions::default_first_use_check`, 모듈 문서)에 다음을 적었다. MC는 동결 "
        "V26 코퍼스(행마다 선언되지 않은 의존이 하나 이하)에서 FA가 잡는 잘못된 선언을 모두 잡지만 일반적으로 정확하지 "
        "않다. (a) 한 행의 선언되지 않은 결합들이 점검 상태에서 모든 채색의 모든 그룹에서 쌍으로 상쇄되면 통과한다. "
        "(b) 항목 붕괴: 행 비트는 바뀌지만 기준값을 빼고 증분으로 나눈 비교 항목이 같은 수로 반올림되면 통과한다. "
        "FA(`All`)나 FA_P(`AllPooled`)가 반올림 한계까지 정확한 선택이다(n번의 우변). 재현: "
        "`research/production_loop_20261116/assembly_check/review_probe`의 `cancel`, `collapse`.")
    add("")
    add("리뷰 probe 로그(L-0338)의 결정 (탐지 = 예):")
    add("")
    add("| probe | 사례 | S8@seq | MC@seq | FA@seq | FA_P@t6 |")
    add("|---|---|---|---|---|---|")
    yes = {True: "예", False: "아니오"}
    for row in results["limits_of_mc_l0338_from_the_review_probe_logs"]:
        add(f"| {row['probe']} | {row['case']} | {yes[row['S8@seq']]} | {yes[row['MC@seq']]} | {yes[row['FA@seq']]} | "
            f"{yes[row['FA_P@t6']]} |")
    add("")
    add("- 소유자에게 알릴 점: 코퍼스에서는 새 기본값이 옛 기본값이 잡는 것을 모두 잡지만, 일반적으로는 상위 집합이 "
        "아니다. 위 항목 붕괴 세 사례는 Sample(8)이 잡고(41열이 표본 열) MC는 놓친다. 이 노드의 단위 테스트 "
        "`the_limits_stated_for_the_default_are_real`가 상쇄 두 구성(B, G)과 붕괴 첫 사례를 기본값으로 다시 확인한다"
        "(기본값 통과, FA와 FA_P 탐지).")
    add("")
    add("## 6. 테스트")
    add("")
    tests = results["tests"]
    add("기존 테스트에서 고친 줄 (기본 검사나 그 카운트를 고정하는 줄만, 이유와 함께):")
    add("")
    add("| 파일 | 줄 | 변경 전 | 변경 후 | 이유 |")
    add("|---|---|---|---|---|")
    for entry in tests["edited_lines_of_existing_tests"]:
        reason = EDIT_REASONS_KO.get((entry["file"], entry["line"]), entry["reason"])
        add(f"| `{entry['file'].split('/')[-1]}` | {entry['line']} | `{entry.get('removed', '').strip()}` | "
            f"`{entry.get('added', '').strip()}` | {reason} |")
    add("")
    add("기존 테스트는 모두 검사를 명시적으로 지정하므로(공통 도우미 `ClientSpec::options`가 항상 "
        "`with_first_use_check`를 부름) 단언문은 하나도 바꿀 필요가 없었다.")
    add("")
    add("새 테스트:")
    add("")
    for path, names in tests["new_test_files"].items():
        add(f"- `{path.split('/')[-1]}`: " + ", ".join(f"`{name}`" for name in names))
    unit = results["unit_tests_of_this_node"] or {}
    if unit:
        add("")
        add(f"`logs/unit.log`: 통과 {unit['passed']}, 실패 {unit['failed']}. 할당(첫 선형화, 순차 채색 "
            "클라이언트): " + "; ".join(
                f"{row['client']}: 기본 검사 {row['first_linearization_allocations_default_check']}, 검사 없음 "
                f"{row['first_linearization_allocations_without_check']}, 나중에 요청한 MC "
                f"{row['first_linearization_allocations_requested_multi_colouring']}"
                for row in unit["allocation"]) + ".")
    add("")
    add("## 7. 점검 (logs/checks.log)")
    add("")
    add("| 점검 | 종료 코드 | 테스트 (통과/실패/무시, 바이너리) | 마지막 줄 |")
    add("|---|---|---|---|")
    for row in (results["checks"] or {}).get("checks", []):
        tally = (f"{row['passed']}/{row['failed']}/{row['ignored']}, {row['binaries']}" if "passed" in row else "-")
        add(f"| {row['name']} | {row['exit_code']} | {tally} | {row['last_line'].replace('|', '/')} |")
    add("")
    checker = results["loop_checker"]
    if checker is None:
        add("루프 점검기: 실행 기록 없음.")
    elif checker["exit_code"] == 0:
        add("루프 점검기: 통과(종료 코드 0).")
    else:
        add(f"루프 점검기의 종료 코드 {checker['exit_code']}은 위 마지막 줄의 불만 때문이다(그대로 옮김: "
            f"`{checker['last_line']}`). 이 노드가 고치도록 허용된 클라이언트 파일의 구현 해시가 CHECKPOINT.json의 "
            "기록과 다르다는 뜻이며, CHECKPOINT.json은 병합 때 조정자가 갱신한다.")
    every = results["loop_checker_every_complaint"]
    if every is not None:
        add("")
        add(f"첫 불만에서 멈추지 않는 사본(`checker_all.py`, `logs/checker_all.log`, HEAD `{(every['git_head'] or '')[:8]}`)의 "
            f"불만 {len(every['complaints'])}건: "
            + "; ".join(f"`{complaint}`" for complaint in every["complaints"])
            + (". 모두 이 노드가 쓰기 허용 목록 안에서 바꾼 고정 구현 파일이다. 보호 파일, 증거 해시, 엄격 JSON, "
               "허용 목록 점검은 통과했다." if every["only_pinned_files_this_node_changes"]
               else ". 이 노드가 바꾼 고정 구현 파일 밖의 불만이 있다."))
    add("")
    add("이 노드가 바꾼 경로: "
        + ", ".join(f"`{path}`" for path in tests["changed_paths_since_base"] if "/logs/" not in path)
        + f" 및 로그 {sum(1 for path in tests['changed_paths_since_base'] if '/logs/' in path)}개; 쓰기 허용 목록 밖: "
        f"{len(tests['paths_outside_the_write_allowlist'])}개.")
    add("")
    add("참고: V08의 파사드 동일성 러너(`jacobian_assembly/runner/src/main.rs:133-142`, 이 노드의 쓰기 경로 밖)는 "
        "검사를 지정하지 않은 기본 옵션을 쓴다. 변경 후 그 밴드 사례는 첫 사용에서 MC를 돌리므로 "
        "`verification_rhs_calls`가 바뀐다. 파사드와의 비트 비교는 위 3절과 같은 이유로 바뀌지 않는다. 이 노드에서는 "
        "다시 돌리지 않았다.")
    add("")
    add("## 8. 프로토콜 형식")
    add("")
    add("- 이 노드는 소유자 결정의 구현이며 실험이나 수치 임계값이 없다. 판별 기준은 계약의 수락 점검(같음 비교)이다.")
    add("- 열려 있던 프로토콜 선택(셀, 팔, 코퍼스, 실행, 두 결정 경로, V26 기준 팔)은 첫 실행 전에 커밋했다: "
        "NOTES.txt 항목 1(`ec284167`, 2026-11-24 23:56 KST)과 기록 프로그램(`f1eb5118`, 2026-11-25 00:03 KST). "
        "첫 실행은 00:03. 형식 편차: PROTOCOL.json이라는 파일은 쓰지 않았다(선택은 NOTES.txt와 프로그램에 있음).")
    add("- 기준 팔: A(모든 경로 Sample(8))는 V26의 S8@seq, S8@t3. B 채색 경로는 seq에 MC@seq, t6과 t3에 MC@t6. "
        "B 조밀 경로는 S8@seq, S8@t3. 조밀 경로 t6은 A와 B를 비교.")
    add("- 결정 경로: first_use = 기본 옵션의 새 클라이언트에서 첫 `linearize`; verify = 기본 옵션의 새 클라이언트에서 "
        "`AssemblyStageClient::verify(t0, y0, options.first_use_check)`(V26 규약).")
    add("")
    add("## 9. 재현")
    add("")
    add("```")
    add("R=research/production_loop_20261116/assembly_default")
    add("$R/run.sh build stage_clients_assembly_default_record")
    add("$R/run.sh identity <A|B> <cell>")
    add("$R/run.sh corpus <A|B> controls seq,t3,t6")
    add("$R/run.sh corpus <A|B> <ring|omit128|narrow|correct|omit1024|omit131072> seq,t6")
    add("$R/run.sh unit")
    add("$R/run.sh checks")
    add("python3 -I $R/parse_results.py")
    add("```")
    add("")
    add("A는 커밋 `f1eb5118`(클라이언트 수정 전)에서, B는 `fc185e2a`(수정 후)에서 같은 프로그램으로 기록했다.")
    add("")
    return "\n".join(lines)


if __name__ == "__main__":
    main()
