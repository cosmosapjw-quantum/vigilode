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


def tagged(path, tag):
    """The JSON objects behind `tag` in a log, in order."""
    rows = []
    prefix = tag + " "
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith(prefix):
                rows.append(json.loads(line[len(prefix):]))
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
                s8_misses.append({"case": ident, "omitted_entries": case["omitted_entries"],
                                  "omitted_first": case["omitted_first"],
                                  "FA@seq": decision_key(arms["FA@seq"]) if "FA@seq" in arms else None})
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
    rows = tagged(path, "V26_MC_UNIT_JSON")
    results = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("test result:"):
                results.append(line.strip())
    return {"log": path.name, "header": header(path), "exit_codes": exit_codes(path),
            "omission_tests": rows, "test_results": results}


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
        derived = {}
        get = lambda action: actions.get(action, {}).get("instructions")
        if get("verify") is not None and get("setup") is not None:
            derived["first_linearization"] = get("verify") - get("setup")
        if get("setup") is not None and get("problem") is not None:
            derived["construction"] = get("setup") - get("problem")
        if get("solve") is not None and get("problem") is not None:
            derived["construction_and_solve"] = get("solve") - get("problem")
        out[cell][arm] = {"actions": actions, "derived": derived}
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
        derived["label"] = "exploratory, diagnostic_only (no quality rule frozen for FA_B)"
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
    }
    results["cost"]["summary"] = cost_summary(results)
    results["cost"]["wall_multiples"] = wall_table(results["cost"]["timing"])
    results["verdicts"] = verdicts(results)
    text = json.dumps(results, indent=1, ensure_ascii=False, allow_nan=False, sort_keys=False)
    (HERE / "RESULTS.json").write_text(text + "\n", encoding="utf-8")
    print("RESULTS.json written")


if __name__ == "__main__":
    main()
