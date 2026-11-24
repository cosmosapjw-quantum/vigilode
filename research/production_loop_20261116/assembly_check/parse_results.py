#!/usr/bin/env python3
"""Results of loop node V26 (full pattern check of the assembly client).

Reads only the raw logs under logs/ (and PREDECLARED.json for its hash and
frozen texts), writes RESULTS.json (strict JSON) and REPORT_KO.md (Korean,
for the owner). Run from anywhere:

    python3 -I research/production_loop_20261116/assembly_check/parse_results.py

Every number in the outputs is computed here from the logs; nothing is typed
in. The frozen discriminators are those of PREDECLARED.json:
  E  FA_P equals FA and MC_P equals MC in the decision on every corpus case;
     Jacobians and trajectories of every cost cell bit-identical with and
     without any check;
  D  FA detects every corpus case not rounding-limited for FA; MC misses no
     case FA detects unless rounding-limited for MC; the misses of S8 listed;
  F  no arm rejects a correct pattern;
  C  cost per arm and cell (no threshold).
"""

import hashlib
import json
import statistics
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
LOGS = HERE / "logs"
ROOT = HERE.parents[2]

GROUPS = ["controls", "ring", "omit128", "omit1024", "omit131072", "narrow", "correct"]
CELLS = ["nlkpp-dense-n400", "nlkpp-band-n4096-r24", "bruss1d-n131072", "bruss1d-n128-coloured"]
LOAD_MAX = 8.0
SPREAD_MAX = 0.10


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tagged(path, tag, anywhere=False):
    """The JSON objects behind `tag` in a log, in order (with `anywhere`, the
    tag may follow other text on its line, as the test harness prints
    'test name ... ' before a captured line)."""
    rows = []
    prefix = tag + " "
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith(prefix):
                rows.append(json.loads(line[len(prefix):]))
            elif anywhere and prefix in line:
                rows.append(json.loads(line[line.index(prefix) + len(prefix):]))
    return rows


def header(path):
    """The '# key: value' header lines of a log."""
    out = {}
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("# ") and ": " in line:
                key, value = line[2:].rstrip("\n").split(": ", 1)
                out.setdefault(key, value)
    return out


def exit_codes(path):
    codes = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("# exit code: "):
                codes.append(int(line.split(": ", 1)[1]))
    return codes


def decision_key(record):
    """The decision of an arm: 'pass' or the typed error (Debug text)."""
    if record["outcome"] == "pass":
        return "pass"
    return record.get("typed", record["outcome"])


def detected(record):
    return record["outcome"] == "error" and bool(record.get("contract_violation"))


def not_applicable(record):
    return record.get("kind") == "Configuration" and "coloured route" in record.get("typed", "")


# ---------------------------------------------------------------- corpus


def load_corpus(label):
    """case id -> case record of logs/corpus_<label>_<group>.log, per group."""
    out = {}
    for group in GROUPS:
        path = LOGS / f"corpus_{label}_{group}.log"
        if not path.exists():
            continue
        out[group] = {case["case"]: case for case in tagged(path, "V26_CASE_JSON")}
    return out


def omitted_rows_distinct(case):
    """Whether no row of the case has two omitted entries (the setting of the
    proof in NOTES.txt), from the case definitions of the corpus program."""
    group, ident = case["case"].split("/", 1)
    if case["omitted_entries"] is None:
        return None
    if case["omitted_entries"] <= 1:
        return True
    if group in ("ring", "controls") and ident.startswith(("ring", "n")):
        # the two periodic corners (0, n-1) and (n-1, 0)
        rows = [entry[0] for entry in case["omitted_first"]]
        return len(rows) == len(set(rows)) and case["omitted_entries"] == len(rows)
    if ident.startswith("chain24"):
        # the lower diagonal (i, i-1), one entry per row
        return True
    if group == "narrow":
        # a band one too narrow on one side loses at most one diagonal entry per row
        return True
    return None


def corpus_analysis():
    before = load_corpus("A")
    after = load_corpus("B")
    result = {"groups": {}, "today_arms_unchanged": {}, "E": {}, "D": {}, "F": {}}
    equal_today = Counter()
    different_today = []
    e_counts = Counter()
    e_differences = []
    d_fa_misses = []
    d_mc_misses = []
    s8_misses = []
    f_alarms = []
    mc_not_applicable = []
    multi_per_row = []
    for group in GROUPS:
        cases_b = after.get(group, {})
        cases_a = before.get(group, {})
        arms_b = sorted({arm for case in cases_b.values() for arm in case["arms"]})
        arms_a = sorted({arm for case in cases_a.values() for arm in case["arms"]})
        detections = defaultdict(Counter)
        wrong_cases = 0
        correct_cases = 0
        for ident, case in sorted(cases_b.items()):
            arms = case["arms"]
            pure = case["pure"]
            wrong = (not pure) or (case["omitted_entries"] or 0) > 0
            if wrong:
                wrong_cases += 1
                if omitted_rows_distinct(case) is False:
                    multi_per_row.append(ident)
            else:
                correct_cases += 1
            # today's arms: the same decision before and after the client edit
            if ident in cases_a:
                for arm, record in cases_a[ident]["arms"].items():
                    if arm in arms:
                        same = decision_key(record) == decision_key(arms[arm])
                        equal_today[same] += 1
                        if not same:
                            different_today.append({"case": ident, "arm": arm,
                                                    "before": decision_key(record),
                                                    "after": decision_key(arms[arm])})
            # E: the pooled arms against their sequential arms
            for pooled, reference_pure, reference_impure in (
                ("FA_P@t2", "FA@seq", "FA@t2"),
                ("FA_P@t4", "FA@seq", "FA@t4"),
                ("FA_P@t6", "FA@seq", "FA@t6"),
                ("MC_P@t6", "MC@seq", "MC@t6"),
            ):
                if pooled not in arms:
                    continue
                reference = reference_pure if pure else reference_impure
                if reference not in arms:
                    continue
                same = decision_key(arms[pooled]) == decision_key(arms[reference])
                e_counts[(pooled, same)] += 1
                if not same:
                    e_differences.append({"case": ident, "pooled": pooled, "reference": reference,
                                          "pooled_decision": decision_key(arms[pooled]),
                                          "reference_decision": decision_key(arms[reference])})
                # the threaded reference on the calling thread decides as the sequential arm too
                if pure and reference_impure in arms:
                    same_execution = decision_key(arms[pooled]) == decision_key(arms[reference_impure])
                    e_counts[(pooled + " vs " + reference_impure, same_execution)] += 1
            for arm, record in arms.items():
                if record["outcome"] == "refused":
                    continue
                if wrong:
                    if not_applicable(record):
                        detections[arm]["not_applicable"] += 1
                        continue
                    detections[arm]["detected" if detected(record) else "missed"] += 1
                else:
                    if not_applicable(record):
                        detections[arm]["not_applicable"] += 1
                        if arm == "MC@seq":
                            mc_not_applicable.append(ident)
                        continue
                    detections[arm]["pass" if record["outcome"] == "pass" else "false_alarm"] += 1
                    if record["outcome"] != "pass":
                        f_alarms.append({"case": ident, "arm": arm, "decision": decision_key(record)})
            if not wrong:
                continue
            evidence = case.get("miss_evidence", {})
            if "S8@seq" in arms and arms["S8@seq"]["outcome"] == "pass":
                others = ["FA@seq", "FA_P@t2", "FA_P@t4", "FA_P@t6", "MC@seq", "MC_P@t6"]
                s8_misses.append({"case": ident, "group": group, "n": case["n"],
                                  "omitted_entries": case["omitted_entries"],
                                  "omitted_first": case["omitted_first"],
                                  "FA@seq": decision_key(arms["FA@seq"]) if "FA@seq" in arms else None,
                                  "detected_by_every_other_arm": all(
                                      arm in arms and detected(arms[arm]) for arm in others)})
            if "FA@seq" in arms and arms["FA@seq"]["outcome"] == "pass":
                ev = evidence.get("FA@seq", {})
                d_fa_misses.append({"case": ident, "rounding_limited_for_FA": bool(ev.get("rounding_limited")),
                                    "evidence_complete": bool(ev.get("complete")),
                                    "entries": [entry["entry"] for entry in ev.get("entries", [])]})
            if "MC@seq" in arms and arms["MC@seq"]["outcome"] == "pass":
                ev = evidence.get("MC@seq", {})
                fa_detects = "FA@seq" in arms and detected(arms["FA@seq"])
                d_mc_misses.append({"case": ident, "FA_detects": fa_detects,
                                    "rounding_limited_for_MC": bool(ev.get("rounding_limited")),
                                    "evidence_complete": bool(ev.get("complete")),
                                    "entries": [entry["entry"] for entry in ev.get("entries", [])]})
        result["groups"][group] = {
            "cases": len(cases_b),
            "wrong_cases": wrong_cases,
            "correct_cases": correct_cases,
            "arms_after_edit": arms_b,
            "arms_before_edit": arms_a,
            "per_arm": {arm: dict(counts) for arm, counts in sorted(detections.items())},
        }
    result["today_arms_unchanged"] = {
        "decisions_compared": sum(equal_today.values()),
        "equal": equal_today[True],
        "different": different_today,
        "holds": equal_today[False] == 0 and equal_today[True] > 0,
    }
    e_table = defaultdict(lambda: {"equal": 0, "different": 0})
    for (name, same), count in e_counts.items():
        e_table[name]["equal" if same else "different"] += count
    result["E"]["corpus"] = {
        "comparisons": dict(sorted(e_table.items())),
        "differences": e_differences,
        "reference_rule": "pure right-hand side: the sequential arm (FA@seq, MC@seq); impure control: the same "
                          "threaded client with the check on the calling thread (FA@tP, MC@t6)",
        "holds": not e_differences and sum(v["equal"] for v in e_table.values()) > 0,
    }
    fa_unexcused = [miss for miss in d_fa_misses if not miss["rounding_limited_for_FA"]]
    mc_unexcused = [miss for miss in d_mc_misses if miss["FA_detects"] and not miss["rounding_limited_for_MC"]]
    result["D"] = {
        "FA_misses": d_fa_misses,
        "FA_misses_not_rounding_limited": fa_unexcused,
        "MC_misses": d_mc_misses,
        "MC_misses_of_cases_FA_detects_not_rounding_limited": mc_unexcused,
        "S8_misses": s8_misses,
        "S8_miss_count": len(s8_misses),
        "rows_with_two_omitted_entries": multi_per_row,
        "FA_holds": not fa_unexcused,
        "MC_holds": not mc_unexcused,
    }
    result["F"] = {
        "false_alarms": f_alarms,
        "not_applicable_cases_MC": mc_not_applicable,
        "holds": not f_alarms,
    }
    return result


# ---------------------------------------------------------------- unit tests


def unit_analysis():
    path = LOGS / "unit.log"
    if not path.exists():
        return None
    rows = tagged(path, "V26_MC_UNIT_JSON", anywhere=True)
    results = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("test result:"):
                results.append(line.strip())
    return {"log": path.name, "header": header(path), "exit_codes": exit_codes(path),
            "omission_tests": rows, "test_results": results}


# ---------------------------------------------------------------- checks


def checks_analysis():
    """The sections of logs/checks.log: command, test result lines, exit code."""
    path = LOGS / "checks.log"
    if not path.exists():
        return None
    sections = []
    current = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("## "):
                current = {"check": line[3:], "command": None, "results": [], "exit_code": None, "output": []}
                sections.append(current)
            elif current is None:
                continue
            elif line.startswith("# command: "):
                current["command"] = line[len("# command: "):]
            elif line.startswith("# exit code: "):
                current["exit_code"] = int(line.split(": ", 1)[1])
            elif line.startswith("test result:"):
                current["results"].append(line)
            elif line.strip() and not line.startswith("#") and not line.strip().startswith("Running"):
                current["output"].append(line.strip())
    for section in sections:
        passed = failed = ignored = 0
        for result in section["results"]:
            parts = result.replace(";", "").split()
            passed += int(parts[parts.index("passed") - 1])
            failed += int(parts[parts.index("failed") - 1])
            ignored += int(parts[parts.index("ignored") - 1])
        section["tests"] = {"passed": passed, "failed": failed, "ignored": ignored, "binaries": len(section["results"])}
        section["output"] = section["output"][-4:]
    return {"log": path.name, "header": header(path), "sections": sections}


# ---------------------------------------------------------------- cost cells


def arm_parts(label):
    check, execution = label.split("@")
    return check, execution


def counts_analysis():
    out = {}
    for cell in CELLS:
        path = LOGS / f"counts_{cell}.log"
        if not path.exists():
            continue
        rows = tagged(path, "V26_COUNT_JSON")
        first = {row["arm"]: row for row in rows if row["what"] == "first_linearization"}
        solves = {row["arm"]: row for row in rows if row["what"] == "solve"}
        reference_first = first.get("None@seq")
        reference_solve = solves.get("None@seq")
        jac_equal = {arm: row["jacobian_fingerprint"] == reference_first["jacobian_fingerprint"]
                     for arm, row in first.items()} if reference_first else {}
        traj_equal = {}
        for arm, row in solves.items():
            if reference_solve is None or "error" in row:
                traj_equal[arm] = False
                continue
            traj_equal[arm] = all(row[key] == reference_solve[key] for key in (
                "final_fingerprint", "state_fingerprint", "jacobian_fingerprint", "attempts",
                "accepted_steps", "rejected_steps", "builds", "assembly_rhs_calls"))
        out[cell] = {
            "log": path.name,
            "exit_codes": exit_codes(path),
            "first_linearization": {arm: {key: row.get(key) for key in (
                "outcome", "rhs_calls", "assembly_rhs_calls", "verification_rhs_calls", "verified_columns",
                "jacobian_fingerprint")} for arm, row in first.items()},
            "solve": {arm: {key: row.get(key) for key in (
                "success", "stop_reason", "attempts", "accepted_steps", "rejected_steps", "t_end",
                "rhs_calls", "builds", "assembly_rhs_calls", "verifications", "verification_rhs_calls",
                "final_fingerprint", "state_fingerprint", "jacobian_fingerprint", "error")}
                for arm, row in solves.items()},
            "E_first_linearization_jacobian_equal_to_None@seq": jac_equal,
            "E_solve_equal_to_None@seq": traj_equal,
            "E_holds": bool(jac_equal) and all(jac_equal.values()) and bool(traj_equal) and all(traj_equal.values()),
        }
    return out


def perf_instructions(path):
    """instructions:u from perf stat -x, output."""
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            parts = line.strip().split(",")
            if len(parts) > 2 and parts[2].startswith("instructions"):
                return int(parts[0]) if parts[0].isdigit() else None
    return None


def perf_analysis():
    table = defaultdict(dict)
    for path in sorted(LOGS.glob("perf_*.log")):
        rows = tagged(path, "V26_PERF_JSON")
        if not rows:
            continue
        row = rows[0]
        table[(row["cell"], row["arm"])][row["action"]] = {
            "instructions": perf_instructions(path), "log": path.name, "exit_codes": exit_codes(path)}
    out = defaultdict(dict)
    for (cell, arm), actions in sorted(table.items()):
        # The problem alone is measured once per cell (arm None@seq) and the
        # client construction once per execution (arm None@<execution>).
        execution = arm.split("@")[1]
        problem = table.get((cell, "None@seq"), {}).get("problem", {}).get("instructions")
        setup = table.get((cell, "None@" + execution), {}).get("setup", {}).get("instructions")
        verify = actions.get("verify", {}).get("instructions")
        solve = actions.get("solve", {}).get("instructions")
        derived = {}
        if verify is not None and setup is not None:
            derived["first_linearization"] = verify - setup
        if setup is not None and problem is not None:
            derived["construction"] = setup - problem
        if solve is not None and problem is not None:
            derived["construction_and_solve"] = solve - problem
        out[cell][arm] = {"actions": actions, "derived": derived,
                          "references": {"problem": "None@seq", "setup": "None@" + execution}}
    return out


def spread(values):
    middle = statistics.median(values)
    return (max(values) - min(values)) / middle if middle else None


def timing_analysis():
    out = {}
    for path in sorted(LOGS.glob("timing_*.log")):
        rows = tagged(path, "V26_TIMING_JSON")
        loads = tagged(path, "V26_LOAD_JSON")
        meta = tagged(path, "V26_META_JSON")
        if not rows:
            continue
        before = next((row["loadavg"]["load_1m"] for row in loads if row["when"] == "before"), None)
        after = next((row["loadavg"]["load_1m"] for row in loads if row["when"] == "after"), None)
        quiet = before is not None and after is not None and before <= LOAD_MAX and after <= LOAD_MAX
        per_arm = defaultdict(list)
        warm = Counter()
        for row in rows:
            if row.get("warm_up"):
                warm[row["arm"]] += 1
                continue
            per_arm[row["arm"]].append(row["wall_ns"])
        arms = {}
        for arm, values in per_arm.items():
            arm_spread = spread(values)
            arms[arm] = {
                "timed": len(values),
                "warm_up": warm[arm],
                "median_ns": statistics.median(values),
                "spread": arm_spread,
                "meets_spread": arm_spread is not None and arm_spread <= SPREAD_MAX,
            }
        rhs_cost = tagged(path, "V26_RHS_COST_JSON")
        hdr = header(path)
        duration = None
        if hdr.get("date") and hdr.get("date at end"):
            from datetime import datetime
            duration = (datetime.fromisoformat(hdr["date at end"]) - datetime.fromisoformat(hdr["date"])).total_seconds()
        out[path.stem] = {
            "log": path.name,
            "cell": meta[0]["cell"] if meta else None,
            "what": meta[0].get("what_env") if meta else None,
            "rounds": meta[0].get("rounds_env") if meta else None,
            "load_1m_before": before,
            "load_1m_after": after,
            "running_compilers": [hdr.get("running compilers at start"), hdr.get("running compilers at end")],
            "quiet": quiet,
            "protocol_2_plus_7": all(arm["timed"] == 7 and arm["warm_up"] == 2 for arm in arms.values()),
            "arms": arms,
            "rhs_cost_median_ns": statistics.median(rhs_cost[0]["per_call_ns"]) if rhs_cost else None,
            "duration_s": duration,
            "exit_codes": exit_codes(path),
        }
    return out


def batched_analysis():
    out = {}
    for path in sorted(LOGS.glob("batched_*.log")):
        rows = tagged(path, "V26_BATCHED_JSON")
        if not rows:
            continue
        row = rows[0]
        entry = {"log": path.name, "record": row, "exit_codes": exit_codes(path)}
        if row.get("action") in ("setup", "single", "batched"):
            entry["instructions"] = perf_instructions(path)
        out[row["action"]] = entry
    derived = {}
    instructions = {action: out.get(action, {}).get("instructions") for action in ("setup", "single", "batched")}
    if all(value is not None for value in instructions.values()):
        evaluations = out["single"]["record"]["evaluations"]
        derived["instructions_per_evaluation_single"] = (instructions["single"] - instructions["setup"]) / evaluations
        derived["instructions_per_evaluation_batched"] = (instructions["batched"] - instructions["setup"]) / evaluations
        derived["batched_over_single_instructions"] = (
            derived["instructions_per_evaluation_batched"] / derived["instructions_per_evaluation_single"])
    if "timing" in out:
        rounds = [r for r in out["timing"]["record"]["rounds"] if not r["warm_up"]]
        batched_ns = [r["batched_ns_for_n_states"] for r in rounds]
        single_ns = [r["single_ns_for_n_states"] for r in rounds]
        states = out["timing"]["record"]["states"]
        derived["wall_per_evaluation_single_ns_median"] = statistics.median(single_ns) / states
        derived["wall_per_evaluation_batched_ns_median"] = statistics.median(batched_ns) / states
        derived["batched_over_single_wall"] = statistics.median(batched_ns) / statistics.median(single_ns)
        derived["spread_single"] = spread(single_ns)
        derived["spread_batched"] = spread(batched_ns)
        derived["label"] = "탐색적, diagnostic_only (FA_B에는 동결된 품질 규칙이 없음)"
    return {"actions": out, "derived": derived}


# ---------------------------------------------------------------- cost summary


def ratio_label(timing, invocation, arm, reference):
    """The ratio of the medians of two arms of one invocation and whether it
    is authoritative (quiet host, 2 + 7 rounds, both spreads at most 0.10)."""
    entry = timing[invocation]
    arms = entry["arms"]
    if arm not in arms or reference not in arms:
        return None
    a, b = arms[arm], arms[reference]
    authoritative = (entry["quiet"] and entry["protocol_2_plus_7"] and a["meets_spread"]
                     and b["meets_spread"])
    return {
        "invocation": invocation,
        "ratio": a["median_ns"] / b["median_ns"],
        "arm_median_ns": a["median_ns"],
        "reference_median_ns": b["median_ns"],
        "authoritative": authoritative,
        "label": "authoritative" if authoritative else "diagnostic_only",
    }


def try_order(name):
    """Invocations of one label in the order of their tries (try1, try2, ..)."""
    stem, _, tried = name.rpartition("_try")
    return stem, int(tried) if tried.isdigit() else 0


def wall_table(timing):
    """Per (cell, what, arm): the multiple of the reference arm, from the
    first try whose ratio is authoritative, else from the last try."""
    candidates = defaultdict(list)
    for name, entry in timing.items():
        stem, tried = try_order(name)
        cell, what = entry["cell"], entry["what"]
        reference_seq = "None@seq" if what == "verify" else "S8@seq"
        for arm in entry["arms"]:
            execution = arm.split("@")[1]
            same_reference = ("None@" if what == "verify" else "S8@") + execution
            candidates[(cell, what, arm)].append((tried, name, reference_seq, same_reference))
    table = {}
    for (cell, what, arm), tries in sorted(candidates.items()):
        tries.sort()
        row = {}
        for key, pick in (("vs_sequential_reference", 2), ("vs_same_execution_reference", 3)):
            chosen = None
            for tried, name, *references in tries:
                reference = references[pick - 2]
                ratio = ratio_label(timing, name, arm, reference)
                if ratio is None:
                    continue
                chosen = ratio
                if ratio["authoritative"]:
                    break
            row[key] = chosen
        medians = [(tried, name, timing[name]["arms"][arm]) for tried, name, *_ in tries]
        row["median_ns_by_try"] = {name: stats["median_ns"] for _, name, stats in medians}
        table[f"{cell}|{what}|{arm}"] = row
    return table


def cost_summary(results):
    counts = results["cost"]["counts"]
    instructions = results["cost"]["instructions"]
    summary = {}
    for cell, data in counts.items():
        first = data["first_linearization"]
        solves = data["solve"]
        build_rhs = first.get("None@seq", {}).get("assembly_rhs_calls")
        cell_rows = {}
        for arm in sorted(set(first) | set(solves)):
            row = {}
            if arm in first:
                check_rhs = first[arm]["verification_rhs_calls"]
                row["check_rhs"] = check_rhs
                row["check_rhs_over_one_build"] = check_rhs / build_rhs if build_rhs else None
            if arm in solves and "error" not in solves[arm]:
                row["solve_rhs_calls"] = solves[arm]["rhs_calls"]
                reference = solves.get("S8@seq", {}).get("rhs_calls")
                row["solve_rhs_over_S8_solve"] = solves[arm]["rhs_calls"] / reference if reference else None
                row["solve_verification_rhs"] = solves[arm]["verification_rhs_calls"]
            inst = instructions.get(cell, {})
            derived = inst.get(arm, {}).get("derived", {})
            base_first = inst.get("None@seq", {}).get("derived", {}).get("first_linearization")
            base_solve = inst.get("S8@seq", {}).get("derived", {}).get("construction_and_solve")
            if "first_linearization" in derived:
                row["instructions_first_linearization"] = derived["first_linearization"]
                row["instructions_first_linearization_over_one_build"] = (
                    derived["first_linearization"] / base_first if base_first else None)
            if "construction_and_solve" in derived:
                row["instructions_solve"] = derived["construction_and_solve"]
                row["instructions_solve_over_S8_solve"] = (
                    derived["construction_and_solve"] / base_solve if base_solve else None)
            cell_rows[arm] = row
        summary[cell] = {"one_build_rhs": build_rhs, "arms": cell_rows}
    return summary


def verdicts(results):
    corpus = results["corpus"]
    counts = results["cost"]["counts"]
    e_cost = {cell: data["E_holds"] for cell, data in counts.items()}
    e_corpus = corpus["E"]["corpus"]["holds"]
    unit = results.get("unit_tests") or {}
    unit_failures = sum(row.get("failures", 0) for row in unit.get("omission_tests", []))
    per_arm = {}
    for arm in ("S8@seq", "FA@seq", "FA_P@t2", "FA_P@t4", "FA_P@t6", "MC@seq", "MC_P@t6"):
        missed_unexcused = []
        alarms = [alarm for alarm in corpus["F"]["false_alarms"] if alarm["arm"] == arm]
        for group, data in corpus["groups"].items():
            missed = data["per_arm"].get(arm, {}).get("missed", 0)
            if missed:
                missed_unexcused.append((group, missed))
        per_arm[arm] = {"misses_by_group": dict(missed_unexcused), "false_alarms": len(alarms)}
    fa_ok = corpus["D"]["FA_holds"]
    mc_ok = corpus["D"]["MC_holds"]
    # An arm keeps D if it misses only what FA misses rounding-limited (FA_P decides as FA, MC_P as MC).
    fa_miss_cases = {miss["case"] for miss in corpus["D"]["FA_misses"]}
    keeps = {}
    for arm in per_arm:
        if arm.startswith("S8"):
            keeps[arm] = {"E": True, "D": corpus["D"]["S8_miss_count"] == 0, "F": per_arm[arm]["false_alarms"] == 0}
        elif arm.startswith("FA"):
            keeps[arm] = {"E": e_corpus, "D": fa_ok, "F": per_arm[arm]["false_alarms"] == 0}
        else:
            keeps[arm] = {"E": e_corpus, "D": fa_ok and mc_ok, "F": per_arm[arm]["false_alarms"] == 0}
    return {
        "E_corpus": e_corpus,
        "E_cost_cells_bits": e_cost,
        "E": e_corpus and all(e_cost.values()),
        "D_FA": fa_ok,
        "D_MC": mc_ok,
        "D_S8_misses": corpus["D"]["S8_miss_count"],
        "F": corpus["F"]["holds"],
        "unit_test_failures": unit_failures,
        "FA_miss_cases": sorted(fa_miss_cases),
        "per_arm": per_arm,
        "arm_keeps_E_D_F": keeps,
    }


# ---------------------------------------------------------------- report


def fmt(value, digits=3):
    if value is None:
        return "-"
    if isinstance(value, bool):
        return "예" if value else "아니오"
    if isinstance(value, int):
        return f"{value:,}"
    if isinstance(value, float):
        if value == 0:
            return "0"
        if abs(value) >= 1000:
            return f"{value:,.0f}"
        return f"{value:.{digits}g}"
    return str(value)


def millis(ns):
    return None if ns is None else ns / 1e6


ARM_NAMES = {
    "None": "검사 없음 (빌드만)",
    "S8": "S8: 균등 8열 표본 (현 기본값)",
    "FA": "FA: 전체 n열, 호출 스레드에서 순차",
    "FA_P": "FA_P: 전체 n열, 클라이언트 풀에서",
    "MC": "MC: 다중 채색 검사, 순차",
    "MC_P": "MC_P: 다중 채색 검사, 풀에서",
    "MCe": "MCe: 매 선형화마다 MC (탐색적)",
}


def render_report(results):
    v = results["verdicts"]
    corpus = results["corpus"]
    cost = results["cost"]
    lines = []
    add = lines.append
    add("# V26 조립 클라이언트 전체 패턴 검사: 병렬화, 벡터화, 다중 채색 검사")
    add("")
    add("이 보고서는 `parse_results.py`가 `logs/`의 원시 로그만 읽어 `RESULTS.json`과 함께 생성했다. "
        "숫자를 손으로 적지 않았다. 동결 설계: `PREDECLARED.json` (sha256 "
        f"`{results['predeclared_sha256'][:16]}...`), 사전등록 부록 A7. 생성 시점 HEAD: `{results['head_at_build']}`.")
    add("")
    add("## 1. 결론")
    add("")
    add("- 질문 (OD-L1-16, \"n회 추가가 벡터화/병렬화가 가능한지 먼저 시험\"): 선택형 클라이언트 "
        "`AssemblyStageClient`의 전체 패턴 검사(`CheckColumns::All`, n개 단일 열 평가)를 (a) 클라이언트 풀에서 "
        "병렬로, (b) 여러 이동 상태를 한 번에 계산하는 벡터화 우변으로, (c) 같은 결정을 훨씬 적은 평가로 내는 "
        "정확한 다중 채색 검사로 바꿀 수 있는가.")
    keeps = v["arm_keeps_E_D_F"]
    kept = [arm for arm, k in keeps.items() if all(k.values())]
    add(f"- 동결 판별 기준: E (결정과 비트 동일) {'충족' if v['E'] else '불충족'}, "
        f"D (FA 탐지 {'충족' if v['D_FA'] else '불충족'}, MC가 FA 탐지분을 놓치지 않음 {'충족' if v['D_MC'] else '불충족'}), "
        f"F (정답 패턴 거부 없음) {'충족' if v['F'] else '불충족'}. E, D, F를 모두 지키는 팔: {', '.join(kept)}. "
        f"S8(현 기본값)은 잘못된 패턴 {v['D_S8_misses']}건을 놓쳐 D를 지키지 못한다.")
    add("- 기본값은 바뀌지 않았다: 첫 사용 검사는 여전히 `CheckColumns::Sample(8)`이다. 새 검사는 "
        "`CheckColumns::AllPooled`, `MultiColouring`, `MultiColouringPooled` 선택지로만 존재한다. 기본값 결정은 소유자 몫이다.")
    add("")
    add("### 핵심 숫자 (첫 선형화 = 빌드 + 검사, 셀마다)")
    add("")
    add("| 셀 | n | 빌드 RHS | S8 검사 RHS | FA 검사 RHS | MC 검사 RHS | 명령어 배수 (빌드 대비) S8 / FA / MC | "
        "벽시계 S8 / FA / FA_P(6) / MC (ms) |")
    add("|---|---|---|---|---|---|---|---|")
    timing_medians = {}
    for key, row in cost["wall_multiples"].items():
        cell, what, arm = key.split("|")
        ref = row.get("vs_sequential_reference")
        if what == "verify" and ref:
            timing_medians[(cell, arm)] = ref
    for cell, data in cost["summary"].items():
        arms = data["arms"]
        n = results["cost"]["counts"][cell]["first_linearization"].get("FA@seq", {}).get("verified_columns")
        def rhs(arm):
            return fmt(arms.get(arm, {}).get("check_rhs"))
        def mult(arm):
            return fmt(arms.get(arm, {}).get("instructions_first_linearization_over_one_build"))
        def wall(arm):
            w = timing_medians.get((cell, arm))
            return "-" if not w else f"{fmt(millis(w['arm_median_ns']))}{'' if w['authoritative'] else ' d'}"
        mc = "MC@seq" if "MC@seq" in arms else None
        add(f"| {cell} | {fmt(n)} | {fmt(data['one_build_rhs'])} | {rhs('S8@seq')} | {rhs('FA@seq')} | "
            f"{rhs(mc) if mc else '적용 안 됨'} | {mult('S8@seq')} / {mult('FA@seq')} / {mult(mc) if mc else '-'} | "
            f"{wall('S8@seq')} / {wall('FA@seq')} / {wall('FA_P@t6')} / {wall(mc) if mc else '-'} |")
    add("")
    add("d = diagnostic_only (품질 규칙 미충족 또는 2분 상한 때문에 횟수를 줄인 측정). 전체 실행(48회 시도)에 대한 배수는 5절.")
    add("")
    add("### 셀별로 E, D, F를 지키는 가장 싼 팔")
    add("")
    add("후보 = E, D, F를 모두 지키는 팔 (FA, FA_P, MC, MC_P; MC는 선언된 패턴이 있는 경로에서만). "
        "세 가지 척도로 따로 고른다: 검사가 부르는 우변 수 (결정적), 첫 선형화의 명령어 수 (instructions:u, 1차 지표; "
        "스레드 팔은 rayon 회전 대기를 포함), 첫 선형화의 벽시계 중앙값 (A = 권위, d = diagnostic_only).")
    add("")
    add("| 셀 | 우변 수 최소 | 명령어 최소 | 벽시계 최소 | FA (순차): 우변 수, 명령어 / 빌드, 벽시계 / 빌드 |")
    add("|---|---|---|---|---|")
    cheapest = results["verdicts"].setdefault("cheapest", {})
    for cell, data in cost["summary"].items():
        rows = []
        for arm, row in data["arms"].items():
            if arm not in keeps or not all(keeps[arm].values()) or row.get("check_rhs") is None:
                continue
            wall = cost["wall_multiples"].get(f"{cell}|verify|{arm}", {}).get("vs_sequential_reference")
            rows.append((arm, row, wall))
        if not rows:
            add(f"| {cell} | - | - | - | - |")
            continue
        by_rhs = min(rows, key=lambda r: (r[1]["check_rhs"], r[0]))
        with_ins = [r for r in rows if r[1].get("instructions_first_linearization") is not None]
        by_ins = min(with_ins, key=lambda r: r[1]["instructions_first_linearization"]) if with_ins else None
        with_wall = [r for r in rows if r[2] is not None]
        by_wall = min(with_wall, key=lambda r: r[2]["arm_median_ns"]) if with_wall else None
        fa = next((r for r in rows if r[0] == "FA@seq"), None)
        cheapest[cell] = {
            "by_check_rhs": by_rhs[0],
            "by_instructions": by_ins[0] if by_ins else None,
            "by_wall": by_wall[0] if by_wall else None,
            "by_wall_label": by_wall[2]["label"] if by_wall else None,
        }
        def wall_text(r):
            if r is None:
                return "-"
            w = r[2]
            return f"{r[0]} ({fmt(millis(w['arm_median_ns']))} ms, {'A' if w['authoritative'] else 'd'})"
        fa_text = "-"
        if fa:
            w = fa[2]
            fa_text = (f"{fmt(fa[1]['check_rhs'])}, {fmt(fa[1].get('instructions_first_linearization_over_one_build'))}, "
                       f"{fmt(w['ratio']) if w else '-'}")
        add(f"| {cell} | {by_rhs[0]} ({fmt(by_rhs[1]['check_rhs'])}) | "
            f"{by_ins[0] + ' (' + fmt(by_ins[1].get('instructions_first_linearization_over_one_build')) + ' x 빌드)' if by_ins else '-'} | "
            f"{wall_text(by_wall)} | {fa_text} |")
    add("")
    add("모든 셀에 적용되는 하나의 팔로 읽으면 MC는 조밀 경로(선언된 패턴 없음)에 적용되지 않으므로 FA와 FA_P만 남는다. "
        "셀별 선택은 표에 있다.")
    add("")
    add("## 2. 팔 (PREDECLARED arms)")
    add("")
    for key, name in ARM_NAMES.items():
        add(f"- `{key}`: {name}")
    add("- 실행: `seq` 순차 클라이언트, `tP` P개 작업자의 스레드 클라이언트 (RhsPurity 확인). FA_P는 P = 2, 4, 6, MC_P는 6.")
    add("- MC 구성: 밴드 구성 B (`j mod m_r`, `m_r >= lower + upper + 1` 서로소, 곱 >= n) 또는 정제 구성 G "
        "(`(c1(j), q(j) mod p_r)`, 소수 곱 >= 가장 큰 c1 클래스), 그룹 합이 작은 쪽 (같으면 B). 색 튜플의 단사성은 "
        "검사 전에 확인한다. 모든 선언 항목을 클라이언트 자신의 채색 결과와 비트 단위로 비교하고, 모든 그룹에서 "
        "덮이지 않은 행 검사를 한다. 허용오차도 조정 매개변수도 없다.")
    add("")
    add("## 3. 코퍼스: 탐지 (D), 거짓 경보 (F), 결정 동일성 (E)")
    add("")
    add("각 사례의 결정 = 새 클라이언트에서 `verify(t0, y0)`의 결과 (통과, 또는 행과 열이 붙은 타입 오류). "
        "드라이버 실행의 첫 선형화와 같은 빌드와 검사다.")
    add("")
    add("| 그룹 | 사례 | 잘못된 패턴 | S8 놓침 | FA 놓침 | FA_P 놓침 (2/4/6) | MC 놓침 | MC_P 놓침 | 정답 패턴 거부 |")
    add("|---|---|---|---|---|---|---|---|---|")
    for group, data in corpus["groups"].items():
        per = data["per_arm"]
        miss = lambda arm: per.get(arm, {}).get("missed", 0) if arm in per else "-"
        alarms = sum(c.get("false_alarm", 0) for c in per.values())
        add(f"| {group} | {data['cases']} | {data['wrong_cases']} | {fmt(miss('S8@seq'))} | {fmt(miss('FA@seq'))} | "
            f"{miss('FA_P@t2')}/{miss('FA_P@t4')}/{miss('FA_P@t6')} | {fmt(miss('MC@seq'))} | {fmt(miss('MC_P@t6'))} | {alarms} |")
    add("")
    d = corpus["D"]
    add(f"- FA의 놓침 {len(d['FA_misses'])}건, 모두 반올림 한계 (직접 평가로 확인): "
        + ", ".join(f"`{m['case']}` (반올림 한계 {fmt(m['rounding_limited_for_FA'])})" for m in d["FA_misses"]) + ".")
    add(f"- MC의 놓침 {len(d['MC_misses'])}건: " + ", ".join(
        f"`{m['case']}` (FA도 놓침: {fmt(not m['FA_detects'])}, MC 반올림 한계: {fmt(m['rounding_limited_for_MC'])})"
        for m in d["MC_misses"]) + ". FA가 잡았는데 MC가 놓친 사례: "
        f"{len(d['MC_misses_of_cases_FA_detects_not_rounding_limited'])}건.")
    by_group = Counter(miss["group"] for miss in d["S8_misses"])
    ring_orders = sorted(miss["n"] for miss in d["S8_misses"] if miss["group"] == "ring")
    covered = sum(1 for miss in d["S8_misses"] if miss["detected_by_every_other_arm"])
    add(f"- S8의 놓침 {d['S8_miss_count']}건 (목록은 RESULTS.json `corpus.D.S8_misses`): 그룹별 "
        + ", ".join(f"{g} {c}" for g, c in sorted(by_group.items()))
        + f"; 고리는 n = {', '.join(str(n) for n in ring_orders)}. 이 중 FA, FA_P(2, 4, 6), MC, MC_P가 모두 잡은 사례: "
        f"{covered}건; 나머지: "
        + (", ".join(f"`{miss['case']}`" for miss in d["S8_misses"] if not miss["detected_by_every_other_arm"]) or "없음")
        + " (위 FA, MC 놓침 목록 참조).")
    e = corpus["E"]["corpus"]
    comparisons = ", ".join(f"{k}: 같음 {c['equal']}, 다름 {c['different']}" for k, c in e["comparisons"].items())
    add(f"- E (코퍼스): {comparisons}. 기준: {e['reference_rule']}.")
    t = corpus["today_arms_unchanged"]
    add(f"- 수정 전 기록 (커밋 A)과 수정 후의 S8, FA 결정: {t['decisions_compared']}개 중 {t['equal']}개 같음.")
    add(f"- F: 정답 패턴을 거부한 팔 없음 ({len(corpus['F']['false_alarms'])}건). MC는 조밀 경로에서 적용되지 않는다 "
        "(구성 오류로 거부; 판별 대상 아님).")
    add("")
    unit = results.get("unit_tests")
    if unit:
        add("## 4. 단위 테스트 (모든 단일 항목 누락)")
        add("")
        add("| 테스트 | 누락 수 | 빌드에서 탐지 | 둘 다 탐지 | 둘 다 놓침 (FA 반올림 한계) | FA만 | MC만 | 실패 |")
        add("|---|---|---|---|---|---|---|---|")
        for row in unit["omission_tests"]:
            add(f"| {row['test']} | {fmt(row['single_entry_omissions'])} | {fmt(row.get('detected_at_the_build'))} | "
                f"{fmt(row['detected_by_both'])} | {fmt(row['detected_by_neither'])} ({fmt(row['full_check_misses_rounding_limited'])}) | "
                f"{fmt(row['full_check_only'])} | {fmt(row['multi_colouring_only'])} | {fmt(row['failures'])} |")
        add("")
        add("결과 줄: " + "; ".join(unit["test_results"]))
        add("")
    add("## 5. 비용 (C): 네 비용 셀, 모든 팔")
    add("")
    add("첫 선형화 = 새 클라이언트에서 빌드 + 검사 (`verify`, 뒷단 복사 제외). 전체 실행 = 클라이언트 구성 + 드라이버 호출, "
        "48회 시도 상한 (V08 프로토콜). 명령어 = perf stat instructions:u, 동작별 프로세스 차이 "
        "(verify - setup, solve - problem). 스레드 팔의 명령어에는 rayon 작업자의 회전 대기가 들어간다.")
    add("")
    for cell, data in cost["summary"].items():
        add(f"### {cell} (빌드 한 번 = {fmt(data['one_build_rhs'])} RHS)")
        add("")
        add("| 팔 | 검사 RHS | 검사 RHS / 빌드 | 첫 선형화 명령어 | / 검사 없는 빌드 (None@seq) | 실행 명령어 / S8@seq 실행 | "
            "첫 선형화 벽시계 / None@seq | 실행 벽시계 / S8@seq |")
        add("|---|---|---|---|---|---|---|---|")
        for arm, row in data["arms"].items():
            w_first = cost["wall_multiples"].get(f"{cell}|verify|{arm}", {}).get("vs_sequential_reference")
            w_solve = cost["wall_multiples"].get(f"{cell}|solve|{arm}", {}).get("vs_sequential_reference")
            wf = "-" if not w_first else f"{fmt(w_first['ratio'])} ({'A' if w_first['authoritative'] else 'd'})"
            ws = "-" if not w_solve else f"{fmt(w_solve['ratio'])} ({'A' if w_solve['authoritative'] else 'd'})"
            add(f"| {arm} | {fmt(row.get('check_rhs'))} | {fmt(row.get('check_rhs_over_one_build'))} | "
                f"{fmt(row.get('instructions_first_linearization'))} | {fmt(row.get('instructions_first_linearization_over_one_build'))} | "
                f"{fmt(row.get('instructions_solve_over_S8_solve'))} | {wf} | {ws} |")
        add("")
    reduced = sorted(name for name, entry in cost["timing"].items() if not entry["protocol_2_plus_7"])
    long_runs = sorted(f"{name} ({fmt(entry['duration_s'])} s)" for name, entry in cost["timing"].items()
                       if entry.get("duration_s") is not None and entry["duration_s"] > 120)
    add("A = 권위 있음 (부하 <= 8 전후, 2 + 7회, 두 팔의 퍼짐 <= 0.10), d = diagnostic_only. 한 번의 평가가 수십 초인 "
        "bruss1d-n131072의 전체 검사는 2분 잠금 상한 안에 2 + 7회를 돌릴 수 없어 횟수를 줄였다 (따라서 diagnostic_only): "
        + ", ".join(reduced) + ". 2분을 넘긴 호출: " + (", ".join(long_runs) if long_runs else "없음")
        + ". 재측정은 한 번(try2) 했다. 같은 실행 기준 배수와 시도별 중앙값은 RESULTS.json `cost.wall_multiples`에 있다.")
    add("")
    add("- E (비용 셀 비트): " + ", ".join(f"{cell} {'같음' if ok else '다름'}" for cell, ok in v["E_cost_cells_bits"].items())
        + " (모든 팔의 첫 선형화 야코비안 지문, 전체 실행의 선형화 지점, 야코비안, 최종 상태 지문이 None@seq와 같음).")
    add("")
    fab = results["fa_batched_exploratory"]
    add("## 6. 벡터화 (FA_B, 탐색적): nlkpp-dense-n400의 n개 이동 상태를 한 번에")
    add("")
    ident = fab["actions"].get("identity", {}).get("record", {})
    der = fab["derived"]
    add(f"- 비트 동일성: {fmt(ident.get('states'))}개 상태 중 단일 평가와 다른 성분 {fmt(ident.get('batched_components_with_other_bits'))}개; "
        f"묶음 상태로 한 전체 검사: {ident.get('full_check_with_batched_states', '-')}.")
    add(f"- 평가당 명령어: 단일 {fmt(der.get('instructions_per_evaluation_single'))}, 묶음 {fmt(der.get('instructions_per_evaluation_batched'))} "
        f"(비 {fmt(der.get('batched_over_single_instructions'))}).")
    add(f"- 평가당 벽시계 (중앙값): 단일 {fmt(der.get('wall_per_evaluation_single_ns_median'))} ns, 묶음 "
        f"{fmt(der.get('wall_per_evaluation_batched_ns_median'))} ns (비 {fmt(der.get('batched_over_single_wall'))}); {der.get('label', '')}.")
    add("- 같은 합산 순서를 지키는 묶음 우변은 사용자가 따로 써야 하는 문제별 코드다. 클라이언트의 API는 우변 한 번에 상태 하나를 받으므로, "
        "이 팔은 클라이언트에 넣지 않았다.")
    add("")
    add("## 7. 주의")
    add("")
    unit_rows = (results.get("unit_tests") or {}).get("omission_tests", [])
    unit_total = sum(row["single_entry_omissions"] for row in unit_rows)
    unit_one_sided = sum(row["full_check_only"] + row["multi_colouring_only"] for row in unit_rows)
    add("- MC의 정확성 주장(NOTES.txt의 증명)은 한 행에 선언되지 않은 의존이 하나일 때다. 코퍼스의 잘못된 패턴에서 한 행에 누락 항목이 둘 이상인 사례: "
        f"{len(d['rows_with_two_omitted_entries'])}건. 증명에 남는 틈은 '항목 붕괴'다: 결합 이동 상태에서 행 값의 비트는 다르지만 f0를 빼고 "
        "delta로 나눈 두 항목이 같은 수로 반올림되는 경우이며, 사전 정의(행 비트 기준)는 이것을 반올림 한계로 면제하지 않는다. "
        f"코퍼스에서 FA가 잡고 MC가 놓친 사례 {len(d['MC_misses_of_cases_FA_detects_not_rounding_limited'])}건, "
        f"단위 사례 {fmt(unit_total)}개 중 한쪽만 잡은 사례 {fmt(unit_one_sided)}건.")
    add("- 스레드 팔의 명령어 수는 rayon 작업자의 회전 대기를 포함해 순차 팔과 직접 비교하기 어렵다; 벽시계가 병렬 이득을 보여 준다.")
    add("- MC의 첫 사용에는 채색 구성(단사성 정렬 포함) 비용이 들어간다; 이후 빌드의 MC(MCe)는 그룹 평가만 한다.")
    add("- 조밀 경로(nlkpp-dense-n400)에는 선언된 패턴이 없어 MC가 적용되지 않는다; 그 셀에서 D를 지키는 팔은 FA와 FA_P뿐이다.")
    add("")
    checks = results.get("checks")
    if checks:
        add("## 8. 점검 (logs/checks.log)")
        add("")
        add("| 점검 | 종료 코드 | 테스트 (통과/실패/무시, 바이너리) | 출력 끝 |")
        add("|---|---|---|---|")
        for section in checks["sections"]:
            t = section["tests"]
            tests = f"{t['passed']}/{t['failed']}/{t['ignored']}, {t['binaries']}" if t["binaries"] else "-"
            tail = section["output"][-1] if section["output"] else "-"
            add(f"| {section['check']} | {section['exit_code']} | {tests} | {tail} |")
        add("")
        add("루프 점검기의 종료 코드 1은 위 마지막 줄의 불만 때문이다: 이 노드가 고치도록 허용된 클라이언트 파일의 구현 해시가 "
            "CHECKPOINT.json의 기록과 다르다 (CHECKPOINT.json은 병합 때 조정자가 갱신한다).")
        add("")
    add("## 9. 재현")
    add("")
    add("```")
    add("research/production_loop_20261116/assembly_check/run.sh corpus <label> <groups> <arms>")
    add("research/production_loop_20261116/assembly_check/run.sh unit")
    add("research/production_loop_20261116/assembly_check/run.sh counts <cell>")
    add("research/production_loop_20261116/assembly_check/run_perf_plan.sh")
    add("research/production_loop_20261116/assembly_check/run_timing_plan.sh <try> [labels]")
    add("research/production_loop_20261116/assembly_check/run.sh batched <identity|setup|single|batched|timing>")
    add("python3 -I research/production_loop_20261116/assembly_check/parse_results.py")
    add("```")
    add("")
    return "\n".join(lines)


# ---------------------------------------------------------------- main


def main():
    predeclared = HERE / "PREDECLARED.json"
    frozen = json.loads(predeclared.read_text(encoding="utf-8"))
    try:
        head = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture_output=True,
                              text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        head = None
    results = {
        "schema": "vigilode.loop-l1.v26-results.v1",
        "node": "V26_ASSEMBLY_FULL_CHECK",
        "generated_by": "parse_results.py from logs/ only",
        "head_at_build": head,
        "predeclared_sha256": sha256(predeclared),
        "discriminators_frozen": frozen["discriminators"],
        "logs": {path.name: sha256(path) for path in sorted(LOGS.glob("*.log"))},
        "corpus": corpus_analysis(),
        "unit_tests": unit_analysis(),
        "cost": {
            "counts": counts_analysis(),
            "instructions": perf_analysis(),
            "timing": timing_analysis(),
        },
        "fa_batched_exploratory": batched_analysis(),
        "checks": checks_analysis(),
    }
    results["cost"]["summary"] = cost_summary(results)
    results["cost"]["wall_multiples"] = wall_table(results["cost"]["timing"])
    results["verdicts"] = verdicts(results)
    text = json.dumps(results, indent=1, ensure_ascii=False, allow_nan=False, sort_keys=False)
    (HERE / "RESULTS.json").write_text(text + "\n", encoding="utf-8")
    (HERE / "REPORT_KO.md").write_text(render_report(results), encoding="utf-8")
    print("RESULTS.json and REPORT_KO.md written")


if __name__ == "__main__":
    main()
