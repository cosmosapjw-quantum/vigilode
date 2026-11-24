#!/usr/bin/env python3
"""V27 phase 2 parser: RESULTS.json and REPORT_KO.md from the raw runner output only.

Inputs (nothing else is read): logs/phase2/RUNS_INDEX.jsonl, the raw stdout files it lists
(logs/phase2/raw/**), and logs/phase2/references.json. The criteria are those of PREDECLARED.json
(sha256 below, frozen in 1ffbda20) as computed in PHASE2_NOTES.txt (sections 5-7 and addendum
0b, committed before the first deciding run); their thresholds are written here as constants.
Strict JSON out (no NaN or Infinity); deterministic (no clock), so a second run gives the same
bytes.

Usage: python3 -I parse_results.py
"""
import hashlib
import json
import math
import os
import re
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(HERE, "logs", "phase2")
INDEX = os.path.join(LOGS, "RUNS_INDEX.jsonl")
REFS = os.path.join(LOGS, "references.json")
OUT_JSON = os.path.join(HERE, "RESULTS.json")
OUT_MD = os.path.join(HERE, "REPORT_KO.md")

PREDECLARED_SHA256 = "325c9d67bccae6e0fc403877e5a80f64c62e62aa7db6dbd63c150716624862ff"
# thresholds of PREDECLARED.json discriminators (frozen)
E3_FACTOR = 2  # cost <= 2 cost(D)
E7_FACTOR = 3  # err(N) <= 3 max(err(D), err(R))
E9_BOUNDS = {"P_heat_fd": 5890, "P_scalar_fd": 181818}  # the V03b bound W
DISCRIMINATOR_TEXT = {
    "E1_economy": "over all cases (problem x tolerance x options) where T and N succeed, excluding kt_vanishing: geometric mean of cost(N)/cost(T) < 1 in callbacks and in operations; and max over cases of cost(N)/min(cost(D), cost(R)) < the same maximum for T, in each unit (min over those of D and R that succeed)",
    "E2_no_new_failure": "no case where T succeeds and N does not, at the same max_steps",
    "E3_neutrality": "every N run without a handover has the final-state bits and the DOPRI5 counters of D plus exactly the pilots' counters, and cost <= 2 cost(D) in each unit; every run decided by a declared fact equals T bit for bit",
    "E4_budget": "attempts over all legs and pilots <= max_steps; exhaustion reports MaxStepsReached{max_steps}",
    "E5_callbacks": "no closure entry before DOPRI5's first stage; no entry with t = t0 except DOPRI5's stages; every entry that is not a stage is a finite-difference column or time-derivative call of a RODAS5P attempt that starts at an accepted state; uncharged entries = 0",
    "E6_determinism": "the three runs have equal digests, counters and bits",
    "E7_accuracy": "err(N) <= 3 max(err(D), err(R)) over those of D and R that succeed, excluding kt_vanishing",
    "E8_callback_cases": "F2 succeeds under N; P_fatal reports CallbackFailed{Rhs, 'sensor offline'}; N2 ends with CallbackFailed at its first pilot (D1)",
    "E9_v03b_bounded_work": "P_heat_fd <= 5890 and P_scalar_fd <= 181818 charged right-hand sides (the V03b bound W), with a recorded handover",
    "E10_pilot_accounting": "pilots <= 1 + floor(log2(A_D / b)) and pilot spend <= DOPRI5 spend in each unit, on every run",
}
FROZEN_DECISION_TEXT = "the Auto route becomes N iff E1 to E10 all pass; otherwise today's route stays (the implementation is kept on its branch and is not merged). E3, E4, E5, E6 or E10 failing refutes the unconditional properties (design or implementation defect); E1 failing refutes the economic claim; E2 failing blocks adoption."
DECLARED_LOSSES_TEXT = "not failures: P_scalar_fd (model 145 vs 74 charged right-hand sides), P_heat_fd (945 vs 723), lin_3400 (687 vs 436), heat_64_fd (3328 vs 2993), short non-stiff runs +0.3 to +26 percent in callbacks (up to +46 percent in operations at rtol 1e-3)"
DECLARED_LOSSES = [("P_scalar_fd", 145, 74), ("P_heat_fd", 945, 723), ("lin_3400", 687, 436),
                   ("heat_64_fd", 3328, 2993)]
NON_STIFF = ["osc_1", "osc_10", "osc_1000", "lotka_volterra", "lorenz", "arenstorf",
             "brusselator_2", "vdp_1", "damped_osc", "nonnormal", "lorenz96_40", "pleiades"]
REPORT_ONLY_E1_E7 = "kt_vanishing"
ARMS = ("T", "N", "D", "R")
MAIN_BINARY = {"T": "T", "N": "N", "D": "N", "R": "N"}


def strict_load(text):
    def bad(constant):
        raise ValueError(f"non-strict JSON constant {constant}")

    return json.loads(text, parse_constant=bad)


def f64(hexbits):
    return struct.unpack(">d", bytes.fromhex(hexbits))[0]


# ----------------------------------------------------------------------------- loading


def load():
    rows = [strict_load(l) for l in open(INDEX) if l.strip()]
    runs = {}  # (case, arm, binary) -> list of lines, in order p1 r1, p1 r2, p2 r1 (p3 r1)
    meta = {}
    order = []
    for row in rows:
        if "stop" in row:
            raise SystemExit(f"the driver stopped: {row}")
        path = os.path.join(HERE, row["stdout_file"])
        text = open(path).read()
        if hashlib.sha256(text.encode()).hexdigest() != row["stdout_sha256"]:
            raise SystemExit(f"stdout changed since the run: {path}")
        lines = [strict_load(l) for l in text.splitlines() if l.strip()]
        key = (row["case"], row["arm"], row["binary"])
        runs.setdefault(key, []).extend(lines)
        if row["case"] not in meta:
            meta[row["case"]] = {"problem": row["problem"], "tolerance": row["tolerance"],
                                 "options": row["options"]}
            order.append(row["case"])
    refs = strict_load(open(REFS).read())["references"]
    return rows, runs, meta, order, refs


# ---------------------------------------------------------------------------- measures


def status_text(arm, run):
    if "error" in run:
        return "error: " + run["error"]
    st = run.get("status")
    if isinstance(st, str):
        return st
    if st.get("status") == "success":
        return "success"
    detail = {k: v for k, v in st.items() if k not in ("status", "kind")}
    return st.get("kind", "failed") + (" " + json.dumps(detail, sort_keys=True) if detail else "")


def measures(line, refs):
    run = line["run"]
    out = {"problem": line["problem"], "arm": line["arm"], "build": line["build"],
           "n": line["n"], "repeat": line["repeat"], "pid": line["pid"]}
    if "error" in run:
        out.update({"success": False, "status": status_text(line["arm"], run), "refusal": run["error"],
                    "cb": None, "op3": None, "attempts": None, "entries": line["entries"],
                    "error": None})
        return out
    c = run["counters"]
    jcb = line["jacobian_callback"]
    kinds = {"rhs": c["rhs_calls"], "jvp": c["jvp_calls"], "ft": c["ft_calls"],
             "jacobian": c["jacobian_builds"] if jcb else 0}
    out.update({
        "success": bool(run["success"]),
        "status": status_text(line["arm"], run),
        "stop": run.get("stop"),
        "cb": sum(kinds.values()),
        "cb_by_kind": kinds,
        "op3": int(run["operations3"]),
        "operations": int(run["operations3"]) / 3,
        "attempts": run["attempts"],
        "dopri5_attempts": run["dopri5_attempts"],
        "rodas5p_attempts": run["rodas5p_attempts"],
        "accepted_steps": run["accepted_steps"],
        "rejected_steps": run["rejected_steps"],
        "final_time_bits": run["final_time_bits"],
        "final_state_bits": run["final_state_bits"],
        "entries": line["entries"],
    })
    # closure entries minus the charged counter of their kind. A positive difference is an
    # uncharged entry (the criterion of E5); a negative one is a counter above the entries
    # (ft_calls counts every time-derivative computation of the driver, also finite-difference
    # ones whose right-hand-side calls are charged as rhs_calls; erratum E5-i of PHASE2_NOTES).
    out["entries_minus_charged"] = {
        "rhs": line["entries"]["rhs"] - c["rhs_calls"],
        "jvp": line["entries"]["jvp"] - c["jvp_calls"],
        "time_derivative": line["entries"]["time_derivative"] - c["ft_calls"],
        "jacobian": line["entries"]["jacobian"] - (c["jacobian_builds"] if jcb else 0),
    }
    out["uncharged"] = {k: max(0, v) for k, v in out["entries_minus_charged"].items()}
    out["charged_without_entry"] = {k: max(0, -v) for k, v in out["entries_minus_charged"].items()}
    if line["arm"] in ("N", "T"):
        out["route_digest"] = run["route_digest"]
        out["analysis_basis"] = run["analysis_basis"]
        out["legs"] = [{"tier": l["tier"], "driver_id": l["driver_id"], "attempts": l["attempts"],
                        "success": l["success"], "t_start_bits": l["t_start_bits"],
                        "t_end_bits": l["t_end_bits"]} for l in run["legs"]]
        handovers = [h for h in run["route_history"]
                     if h["reason"].startswith("handover at t = ")
                     or h["reason"].startswith("DOPRI5 stopped at t = ")]
        first = run["legs"][0] if run["legs"] else None
        out["handover"] = None
        if handovers:
            h = handovers[0]
            out["handover"] = {
                "kind": "accepted-pilot" if h["reason"].startswith("handover") else "dopri5-stop",
                "time": h["time"], "time_bits": struct.pack(">d", h["time"]).hex(),
                "dopri5_attempts_before": first["attempts"] if first and first["tier"] == "tier1-explicit" else None,
                "reason": h["reason"],
            }
    if line["arm"] == "N":
        pilots = run.get("pilots") or []
        out["pilots"] = [{"index": p["index"], "verdict": p["verdict"], "dopri5_attempts": p["dopri5_attempts"],
                          "t": p["t"], "h": p["h"], "window_attempts": p["window_attempts"],
                          "window_callbacks": p["window_callbacks"],
                          "window_operations": p["window_operations"],
                          "pilot_attempts": p["pilot_attempts"],
                          "pilot_callbacks": p["pilot_callbacks"],
                          "pilot_operations": p["pilot_operations"]} for p in pilots]
        out["pilot_summary"] = {
            "count": len(pilots),
            "lost": sum(p["verdict"] == "lost" for p in pilots),
            "accepted": sum(p["verdict"] == "accepted" for p in pilots),
            "fatal": sum(p["verdict"] == "fatal" for p in pilots),
            "refused": sum(p["verdict"] == "refused" for p in pilots),
            "pilot_callbacks": sum(p["pilot_callbacks"] for p in pilots),
            "pilot_operations3": sum(round(3 * p["pilot_operations"]) for p in pilots),
            "pilot_attempts": sum(p["pilot_attempts"] for p in pilots),
            "window_callbacks": sum(p["window_callbacks"] for p in pilots),
            "window_operations3": sum(round(3 * p["window_operations"]) for p in pilots),
        }
    # error at t1 against the reference
    out["error"] = None
    ref = refs.get(line["problem"])
    t1_bits = line["span_bits"][1]
    if out["success"] and ref and ref.get("y") is not None and out["final_time_bits"] == t1_bits:
        rtol = line["options"]["rtol"]
        atol = line["options"]["atol"]["value"]
        yref = [f64(b) for b in ref["y_bits"]]
        y = [f64(b) for b in out["final_state_bits"]]
        s = sum(((a - r) / (atol + rtol * abs(r))) ** 2 for a, r in zip(y, yref))
        out["error"] = math.sqrt(s / len(y))
    return out


def essence(line):
    run = line["run"]
    keys = ("final_time_bits", "final_state_bits", "status", "success", "counters", "attempts",
            "accepted_steps", "rejected_steps", "stop", "error", "route_digest")
    return json.dumps({"run": {k: run.get(k) for k in keys}, "entries": line["entries"]},
                      sort_keys=True)


def sanity_essence(line):
    run = line["run"]
    keys = ("final_time_bits", "final_state_bits", "status", "success", "counters", "attempts",
            "accepted_steps", "rejected_steps", "stop", "error")
    return json.dumps({"run": {k: run.get(k) for k in keys}, "entries": line["entries"]},
                      sort_keys=True)


# ------------------------------------------------------------------------ discriminators


def geomean(values):
    return math.exp(sum(math.log(v) for v in values) / len(values)) if values else None


def largest_doubling(a_d, b):
    """K = max k >= 0 with b 2^k <= a_d, or -1 if a_d < b."""
    k = -1
    while b * 2 ** (k + 1) <= a_d:
        k += 1
    return k


def evaluate(runs, meta, order, refs):
    cases = {}
    for cid in order:
        row = {"case": cid, **meta[cid], "arms": {}, "runs": {}}
        for arm in ARMS:
            lines = runs.get((cid, arm, MAIN_BINARY[arm]), [])
            row["runs"][arm] = lines
            row["arms"][arm] = measures(lines[0], refs) if lines else None
        row["sanity_lines"] = {arm: runs.get((cid, arm, "T"), []) for arm in ("D", "R")}
        cases[cid] = row

    def ok(row, arm):
        m = row["arms"][arm]
        return bool(m and m["success"])

    res = {}
    # sanity (PREDECLARED arms.sanity)
    sanity = []
    for cid, row in cases.items():
        for arm in ("D", "R"):
            a, b = row["runs"][arm], row["sanity_lines"][arm]
            equal = bool(a and b and sanity_essence(a[0]) == sanity_essence(b[0]))
            if not equal:
                sanity.append({"case": cid, "arm": arm})
    res["sanity"] = {"rule": "D and R built from both checkouts must agree bit for bit",
                     "compared": 2 * len(cases), "disagreements": sanity,
                     "verdict": "PASS" if not sanity else "FAIL"}

    # E1
    c1 = [cid for cid, row in cases.items()
          if ok(row, "T") and ok(row, "N") and row["problem"] != REPORT_ONLY_E1_E7]
    per = []
    for cid in c1:
        row = cases[cid]
        t, n = row["arms"]["T"], row["arms"]["N"]
        best = [row["arms"][a] for a in ("D", "R") if ok(row, a)]
        entry = {"case": cid, "cb_N_over_T": n["cb"] / t["cb"], "op_N_over_T": n["op3"] / t["op3"]}
        if best:
            bcb = min(m["cb"] for m in best)
            bop = min(m["op3"] for m in best)
            entry.update({"best_single_cb": bcb, "best_single_op3": bop,
                          "cb_N_over_best": n["cb"] / bcb, "cb_T_over_best": t["cb"] / bcb,
                          "op_N_over_best": n["op3"] / bop, "op_T_over_best": t["op3"] / bop})
        per.append(entry)
    g_cb = geomean([e["cb_N_over_T"] for e in per])
    g_op = geomean([e["op_N_over_T"] for e in per])

    def worst(key):
        rows_ = [e for e in per if key in e]
        if not rows_:
            return None, None
        top = max(rows_, key=lambda e: (e[key], e["case"]))
        return top[key], top["case"]

    m_cb_n, m_cb_n_case = worst("cb_N_over_best")
    m_cb_t, m_cb_t_case = worst("cb_T_over_best")
    m_op_n, m_op_n_case = worst("op_N_over_best")
    m_op_t, m_op_t_case = worst("op_T_over_best")
    e1_parts = {"geomean_cb_below_1": g_cb is not None and g_cb < 1,
                "geomean_op_below_1": g_op is not None and g_op < 1,
                "max_cb_N_below_T": m_cb_n is not None and m_cb_t is not None and m_cb_n < m_cb_t,
                "max_op_N_below_T": m_op_n is not None and m_op_t is not None and m_op_n < m_op_t}
    undeclared = [e for e in per
                  if cases[e["case"]]["arms"]["N"].get("analysis_basis", {}).get("kind") == "break-even"]
    tied = [e["case"] for e in per if "cb_N_over_best" in e
            and e["cb_N_over_best"] == m_cb_n and e["cb_T_over_best"] == m_cb_t]

    def worst_of(rows_, key):
        rows_ = [e for e in rows_ if key in e]
        top = max(rows_, key=lambda e: (e[key], e["case"])) if rows_ else None
        return {"value": top[key], "case": top["case"]} if top else None

    descriptive = {
        "note": "descriptive only, not a criterion: the maxima over the cases that N runs on the break-even route (no declared fact)",
        "cases": len(undeclared),
        "max_cb_N_over_best_single": worst_of(undeclared, "cb_N_over_best"),
        "max_cb_T_over_best_single": worst_of(undeclared, "cb_T_over_best"),
        "max_op_N_over_best_single": worst_of(undeclared, "op_N_over_best"),
        "max_op_T_over_best_single": worst_of(undeclared, "op_T_over_best"),
        "cases_where_the_maxima_of_N_and_T_are_attained_together": tied,
    }
    res["E1_economy"] = {
        "descriptive_break_even_route_only": descriptive,
        "cases": len(per),
        "cases_with_a_successful_single_method": sum("cb_N_over_best" in e for e in per),
        "geomean_cb_N_over_T": g_cb, "geomean_op_N_over_T": g_op,
        "max_cb_N_over_best_single": m_cb_n, "max_cb_N_over_best_single_case": m_cb_n_case,
        "max_cb_T_over_best_single": m_cb_t, "max_cb_T_over_best_single_case": m_cb_t_case,
        "max_op_N_over_best_single": m_op_n, "max_op_N_over_best_single_case": m_op_n_case,
        "max_op_T_over_best_single": m_op_t, "max_op_T_over_best_single_case": m_op_t_case,
        "parts": e1_parts,
        "per_case": per,
        "verdict": "PASS" if all(e1_parts.values()) else "FAIL",
    }

    # E2
    new_fail = [cid for cid, row in cases.items() if ok(row, "T") and not ok(row, "N")]
    res["E2_no_new_failure"] = {
        "T_succeeds": sum(ok(r, "T") for r in cases.values()),
        "N_succeeds": sum(ok(r, "N") for r in cases.values()),
        "T_succeeds_N_not": new_fail,
        "N_succeeds_T_not": [cid for cid, r in cases.items() if ok(r, "N") and not ok(r, "T")],
        "both_fail": [cid for cid, r in cases.items() if not ok(r, "N") and not ok(r, "T")],
        "verdict": "PASS" if not new_fail else "FAIL",
    }

    # E3
    e3a, e3b = [], []
    for cid, row in cases.items():
        line = row["runs"]["N"][0]
        run = line["run"]
        n = row["arms"]["N"]
        if "error" in run:
            continue
        legs = run["legs"]
        if run["analysis_basis"]["kind"] == "break-even":
            if len(legs) == 1 and legs[0]["tier"] == "tier1-explicit":
                d_line = row["runs"]["D"][0]
                d = row["arms"]["D"]
                ps = n["pilot_summary"]
                if "error" in d_line["run"]:
                    e3a.append({"case": cid, "holds": False, "why": "D refused"})
                    continue
                dc = d_line["run"]["counters"]
                nc = run["counters"]
                checks = {
                    "final_time_bits_equal_D": n["final_time_bits"] == d["final_time_bits"],
                    "final_state_bits_equal_D": n["final_state_bits"] == d["final_state_bits"],
                    "dopri5_leg_counters_equal_D": legs[0]["counters"] == dc,
                    "dopri5_leg_attempts_equal_D": legs[0]["attempts"] == d["attempts"],
                    "rhs_ft_equal_D_plus_pilots": nc["rhs_calls"] + nc["ft_calls"]
                    == dc["rhs_calls"] + dc["ft_calls"] + ps["pilot_callbacks"],
                    "attempts_equal_D_plus_pilots": n["attempts"] == d["attempts"] + ps["pilot_attempts"],
                    "op3_equal_D_plus_pilots": n["op3"] == d["op3"] + ps["pilot_operations3"],
                    "cb_at_most_2_D": n["cb"] <= E3_FACTOR * d["cb"],
                    "op_at_most_2_D": n["op3"] <= E3_FACTOR * d["op3"],
                }
                ending = ("fatal pilot (D1)" if ps["fatal"] else
                          "budget" if (not n["success"] and "max-steps" in n["status"]) else
                          "completed" if n["success"] else "DOPRI5 stopped: " + n["status"])
                e3a.append({"case": cid, "ending": ending, "pilots": ps["count"],
                            "cb_N": n["cb"], "cb_D": d["cb"], "op3_N": n["op3"], "op3_D": d["op3"],
                            "checks": checks, "holds": all(checks.values())})
        else:
            t_line = row["runs"]["T"][0]
            t = t_line["run"]
            if "error" in t:
                e3b.append({"case": cid, "holds": False, "why": "T refused, N did not"})
                continue
            fields = ("final_time_bits", "final_state_bits", "success", "status", "stop",
                      "counters", "attempts", "accepted_steps", "rejected_steps")
            diff = [k for k in fields if run.get(k) != t.get(k)]
            if line["entries"] != t_line["entries"]:
                diff.append("entries")
            e3b.append({"case": cid, "basis": run["analysis_basis"]["kind"],
                        "differing_fields": diff, "holds": not diff,
                        "route_digest_equal": run["route_digest"] == t["route_digest"]})
    e3a_fail = [e["case"] for e in e3a if not e["holds"]]
    e3b_fail = [e["case"] for e in e3b if not e["holds"]]
    res["E3_neutrality"] = {
        "a_runs_without_handover": len(e3a), "a_failing": e3a_fail,
        "a_failing_by_ending": {k: [e["case"] for e in e3a if not e["holds"] and e.get("ending") == k]
                                for k in sorted({e.get("ending") for e in e3a if not e["holds"]} - {None})},
        "a_holds_without_runs_ended_by_a_fatal_pilot_descriptive":
            all(e["holds"] for e in e3a if e.get("ending") != "fatal pilot (D1)"),
        "b_runs_decided_by_a_declared_fact": len(e3b), "b_failing": e3b_fail,
        "a": e3a, "b": e3b,
        "reading": "R1 of PHASE2_NOTES.txt: evaluated as written over every run without a handover, including runs ended by a fatal pilot (D1); the descriptive split is not a substitute criterion",
        "verdict": "PASS" if not e3a_fail and not e3b_fail else "FAIL",
    }

    # E4
    e4 = []
    for cid, row in cases.items():
        line = row["runs"]["N"][0]
        run = line["run"]
        if "error" in run:
            continue
        max_steps = line["options"]["max_steps"]
        legs_att = sum(l["attempts"] for l in run["legs"])
        spare = sum(p["pilot_attempts"] for p in (run.get("pilots") or []) if p["verdict"] != "accepted")
        total = legs_att + spare
        st = run["status"]
        budget_stop = isinstance(st, dict) and st.get("kind") == "max-steps-reached"
        typed = (not budget_stop) or st.get("max_steps") == max_steps
        exhausted = (not run["success"]) and total >= max_steps
        checks = {"attempts_within_max_steps": total <= max_steps,
                  "attempts_equal_record": total == run["attempts"],
                  "budget_stop_reports_max_steps": typed,
                  "exhaustion_is_typed": (not exhausted) or (budget_stop and typed)}
        e4.append({"case": cid, "max_steps": max_steps, "attempts_all": total, "checks": checks,
                   "holds": all(checks.values())})
    e4_fail = [e["case"] for e in e4 if not e["holds"]]
    res["E4_budget"] = {"runs": len(e4), "failing": e4_fail,
                        "tightest": sorted(({"case": e["case"], "attempts_all": e["attempts_all"],
                                             "max_steps": e["max_steps"]} for e in e4),
                                           key=lambda e: (-e["attempts_all"] / e["max_steps"], e["case"]))[:3],
                        "per_run": e4, "verdict": "PASS" if not e4_fail else "FAIL"}

    # E5
    e5 = []
    for cid, row in cases.items():
        line = row["runs"]["N"][0]
        run = line["run"]
        n = row["arms"]["N"]
        uncharged = n["uncharged"] if "uncharged" in n else None
        i_ok = uncharged is not None and all(v == 0 for v in uncharged.values())
        diff = n.get("entries_minus_charged")
        entry = {"case": cid, "uncharged": uncharged, "i_uncharged_zero": i_ok,
                 "entries_minus_charged": diff,
                 "i_as_first_noted_all_differences_zero": diff is not None
                 and all(v == 0 for v in diff.values())}
        split = line.get("e5")
        if "error" in run or not split or not split.get("applicable"):
            entry.update({"ii_iii": "not applicable (the route did not start with DOPRI5)"})
            entry["holds"] = i_ok
            entry["holds_under_first_noted_formula"] = entry["i_as_first_noted_all_differences_zero"]
        else:
            d_kind = row["runs"]["D"][0]["entries"]["rhs_by_kind"]
            k = line["entries"]["rhs_by_kind"]
            parts = split["parts"]
            ii = {"first_is_t0_y0": k["first_is_t0_y0"],
                  "first_entry_is_dopri5_solve_first": split["first_entry_is_dopri5_solve_first"],
                  "at_t0_equal_D": k["at_t0"] == d_kind["at_t0"],
                  "at_t0_not_y0_equal_D": k["at_t0_not_y0"] == d_kind["at_t0_not_y0"],
                  "pilots_at_t0_zero": parts["pilots"]["at_t0"] == 0,
                  "rodas5p_run_at_t0_zero": parts["rodas5p_run"]["at_t0"] == 0}
            iii = {"split": bool(split["split"]),
                   "pilot_starts_are_dopri5_accepted_points": all(
                       p.get("start_equals_record") and p.get("start_after_t0") for p in split["pilots"]),
                   "dopri5_part_has_no_fd_entry": parts["dopri5"]["fd_column"] == 0
                   and parts["dopri5"]["fd_time_derivative"] == 0}
            entry.update({"ii": ii, "iii": iii, "mismatch": split["mismatch"],
                          "parts": parts, "rodas5p_run": split["rodas5p_run"],
                          "dopri5_entries": split["dopri5_entries"],
                          "dopri5_is_all_of_dopri5_solve": split["dopri5_is_all_of_dopri5_solve"]})
            entry["holds"] = i_ok and all(ii.values()) and all(iii.values())
            entry["holds_under_first_noted_formula"] = (entry["i_as_first_noted_all_differences_zero"]
                                                        and all(ii.values()) and all(iii.values()))
        e5.append(entry)
    e5_fail = [e["case"] for e in e5 if not e["holds"]]
    # the formula first written in PHASE2_NOTES 6 (every difference 0), kept for the record
    first = [e["case"] for e in e5 if not e["holds_under_first_noted_formula"]]
    charged_without_entry = {}
    for cid, row in cases.items():
        for arm in ARMS:
            m = row["arms"][arm]
            extra = {k: v for k, v in (m.get("charged_without_entry") or {}).items() if v}
            if extra:
                charged_without_entry.setdefault(cid, {})[arm] = extra
    res["E5_callbacks"] = {"runs": len(e5),
                           "runs_starting_with_dopri5": sum("ii" in e for e in e5),
                           "failing": e5_fail, "per_run": e5,
                           "erratum_E5_i": {
                               "first_noted_formula": "entries - charged counter = 0 for rhs, jvp, time derivative (ft_calls), jacobian",
                               "failing_under_first_noted_formula": first,
                               "charged_without_entry_by_case_and_arm": charged_without_entry,
                               "why_not_the_criterion": "the frozen criterion is 'uncharged entries = 0' (entries that no counter charges); ft_calls also counts time-derivative computations without a time-derivative callback (finite-difference ones, whose right-hand-side calls are charged as rhs_calls, and those of P_mass), in arms N, R and T alike; a counter above the entries is not an uncharged entry",
                               "verdict_under_first_noted_formula": "PASS" if not first else "FAIL",
                           },
                           "verdict": "PASS" if not e5_fail else "FAIL"}

    # E6
    e6 = []
    for cid, row in cases.items():
        for arm in ARMS:
            lines = row["runs"][arm]
            same = len(lines) == 3 and len({essence(l) for l in lines}) == 1
            pids = [l["pid"] for l in lines]
            fresh = len(pids) == 3 and pids[0] == pids[1] and pids[2] != pids[0]
            e6.append({"case": cid, "arm": arm, "runs": len(lines), "equal": same,
                       "two_in_one_process_and_one_fresh": fresh})
    e6_fail = [f"{e['case']}/{e['arm']}" for e in e6 if not (e["equal"] and e["two_in_one_process_and_one_fresh"])]
    res["E6_determinism"] = {"case_arms": len(e6), "failing": e6_fail,
                             "verdict": "PASS" if not e6_fail else "FAIL"}

    # E7
    e7 = []
    for cid, row in cases.items():
        if row["problem"] == REPORT_ONLY_E1_E7:
            continue
        n = row["arms"]["N"]
        if not (n["success"] and n["error"] is not None):
            continue
        refs_err = {a: row["arms"][a]["error"] for a in ("D", "R")
                    if ok(row, a) and row["arms"][a]["error"] is not None}
        if not refs_err:
            continue
        bound = E7_FACTOR * max(refs_err.values())
        yref = [f64(x) for x in refs[row["problem"]]["y_bits"]]
        abs_err = {a: max(abs(f64(x) - r) for x, r in zip(row["arms"][a]["final_state_bits"], yref))
                   for a in ("N", "D", "R") if row["arms"][a]["error"] is not None}
        e7.append({"case": cid, "max_abs_error": abs_err,
                   "err_N": n["error"], "err_D": refs_err.get("D"),
                   "err_R": refs_err.get("R"), "bound": bound, "holds": n["error"] <= bound,
                   "ratio_to_max": n["error"] / max(refs_err.values()) if max(refs_err.values()) > 0 else None})
    e7_fail = [e["case"] for e in e7 if not e["holds"]]
    worst7 = max((e for e in e7 if e["ratio_to_max"] is not None),
                 key=lambda e: (e["ratio_to_max"], e["case"]), default=None)
    res["E7_accuracy"] = {"cases": len(e7), "failing": e7_fail,
                          "worst_ratio": worst7["ratio_to_max"] if worst7 else None,
                          "worst_case": worst7["case"] if worst7 else None,
                          "per_case": e7, "verdict": "PASS" if not e7_fail else "FAIL"}

    # E8
    def n_of(cid):
        return cases[cid]["runs"]["N"][0]["run"], cases[cid]["arms"]["N"]

    f2_run, f2 = n_of("F2|A|default")
    pf_run, pf = n_of("P_fatal|A|default")
    n2_run, n2 = n_of("N2|A|default")
    pf_status = pf_run.get("status") or {}
    n2_status = n2_run.get("status") or {}
    n2_pilots = n2_run.get("pilots") or []
    e8 = {
        "F2_succeeds": bool(f2["success"]),
        "P_fatal_callback_failed_rhs_sensor_offline": isinstance(pf_status, dict)
        and pf_status.get("kind") == "callback-failed" and pf_status.get("callback") == "rhs"
        and pf_status.get("message") == "sensor offline",
        "N2_callback_failed_at_first_pilot": isinstance(n2_status, dict)
        and n2_status.get("kind") == "callback-failed" and not n2["success"]
        and len(n2_pilots) == 1 and n2_pilots[0]["verdict"] == "fatal" and n2_pilots[0]["index"] == 0,
    }
    res["E8_callback_cases"] = {
        "checks": e8,
        "F2": {"status": f2["status"], "cb": f2["cb"], "attempts": f2["attempts"]},
        "P_fatal": {"status": pf["status"], "stop": pf.get("stop")},
        "N2": {"status": n2["status"], "pilots": n2.get("pilots"), "cb": n2["cb"]},
        "verdict": "PASS" if all(e8.values()) else "FAIL",
    }

    # E9
    e9 = {}
    for problem, bound in E9_BOUNDS.items():
        for tol in "ALS":
            cid = f"{problem}|{tol}|default"
            if cid not in cases:
                continue
            n = cases[cid]["arms"]["N"]
            rhs = cases[cid]["runs"]["N"][0]["run"]["counters"]["rhs_calls"]
            e9[cid] = {"charged_rhs": rhs, "bound": bound, "handover": n["handover"],
                       "holds": n["success"] and rhs <= bound and bool(n["handover"]),
                       "decides": tol == "A"}
    e9_fail = [cid for cid, e in e9.items() if e["decides"] and not e["holds"]]
    res["E9_v03b_bounded_work"] = {"per_case": e9, "failing": e9_fail,
                                   "reported_without_rule_failing": [cid for cid, e in e9.items() if not e["decides"] and not e["holds"]],
                                   "verdict": "PASS" if not e9_fail else "FAIL"}

    # E10
    e10 = []
    no_pilot = 0
    for cid, row in cases.items():
        line = row["runs"]["N"][0]
        run = line["run"]
        if "error" in run:
            continue
        pilots = run.get("pilots") or []
        if not pilots:
            no_pilot += 1
            continue
        legs = run["legs"]
        a_d = legs[0]["attempts"] if legs and legs[0]["tier"] == "tier1-explicit" else 0
        b = run["analysis_basis"].get("b")
        k = largest_doubling(a_d, b)
        bound = 1 + k
        n_dim = line["n"]
        spend_cb = sum(p["pilot_callbacks"] for p in pilots)
        spend_op3 = sum(round(3 * p["pilot_operations"]) for p in pilots)
        d_cb = legs[0]["counters"]["rhs_calls"]
        d_op3 = 156 * n_dim * a_d
        checks = {"count_within_bound": len(pilots) <= bound,
                  "spend_cb_within_dopri5": spend_cb <= d_cb,
                  "spend_op_within_dopri5": spend_op3 <= d_op3}
        e10.append({"case": cid, "pilots": len(pilots), "A_D": a_d, "b": b, "bound": bound,
                    "pilot_cb": spend_cb, "dopri5_cb": d_cb, "pilot_op3": spend_op3,
                    "dopri5_op3": d_op3, "checks": checks, "holds": all(checks.values())})
    e10_fail = [e["case"] for e in e10 if not e["holds"]]
    res["E10_pilot_accounting"] = {"runs_with_pilots": len(e10), "runs_without_pilot": no_pilot,
                                   "failing": e10_fail, "per_run": e10,
                                   "max_spend_ratio_cb": max((e["pilot_cb"] / e["dopri5_cb"] for e in e10), default=None),
                                   "max_spend_ratio_op": max((e["pilot_op3"] / e["dopri5_op3"] for e in e10), default=None),
                                   "reading": "R6 of PHASE2_NOTES.txt addendum 0b: evaluated on every N run with at least one pilot",
                                   "verdict": "PASS" if not e10_fail else "FAIL"}
    return cases, res


# -------------------------------------------------------------------------- the losses


def losses(cases):
    out = []
    for problem, model_n, model_t in DECLARED_LOSSES:
        row = cases.get(f"{problem}|A|default")
        n, t = row["arms"]["N"], row["arms"]["T"]
        out.append({"case": f"{problem}|A|default", "declared_model_N_cb": model_n,
                    "declared_model_T_cb": model_t, "measured_N_cb": n["cb"], "measured_T_cb": t["cb"],
                    "measured_N_op3": n["op3"], "measured_T_op3": t["op3"],
                    "measured_N_charged_rhs": n["cb_by_kind"]["rhs"],
                    "measured_T_charged_rhs": t["cb_by_kind"]["rhs"]})
    ns = {}
    for tol in "ALS":
        rows = [cases[f"{p}|{tol}|default"] for p in NON_STIFF]
        cb = [r["arms"]["N"]["cb"] / r["arms"]["T"]["cb"] - 1 for r in rows
              if r["arms"]["N"]["success"] and r["arms"]["T"]["success"]]
        op = [r["arms"]["N"]["op3"] / r["arms"]["T"]["op3"] - 1 for r in rows
              if r["arms"]["N"]["success"] and r["arms"]["T"]["success"]]
        losses_cb = [x for x in cb if x > 0]
        losses_op = [x for x in op if x > 0]
        ns[tol] = {"cases": len(cb), "cb_increase_cases": len(losses_cb),
                   "cb_increase_min": min(losses_cb, default=None),
                   "cb_increase_max": max(losses_cb, default=None),
                   "op_increase_cases": len(losses_op),
                   "op_increase_max": max(losses_op, default=None)}
    return {"declared": DECLARED_LOSSES_TEXT, "named_cases": out, "non_stiff_by_tolerance": ns}


# ---------------------------------------------------------------------------- outputs


def case_table(cases):
    table = []
    for cid, row in cases.items():
        arms = {}
        for arm in ARMS:
            m = dict(row["arms"][arm])
            lines = row["runs"][arm]
            m["three_runs"] = [{"final_state_bits": l["run"].get("final_state_bits"),
                                "final_time_bits": l["run"].get("final_time_bits"),
                                "route_digest": l["run"].get("route_digest"), "pid": l["pid"]}
                               for l in lines]
            for k in ("repeat", "pid"):
                m.pop(k, None)
            arms[arm] = m
        table.append({"case": cid, "problem": row["problem"], "tolerance": row["tolerance"],
                      "options": row["options"], "arms": arms})
    return table


def fmt(x, digits=4):
    if x is None:
        return "-"
    if isinstance(x, bool):
        return "예" if x else "아니오"
    if isinstance(x, int):
        return str(x)
    if x == 0:
        return "0"
    return f"{x:.{digits}g}"


def report(results):
    d = results["discriminators"]
    lines = []
    w = lines.append
    w("# V27 phase 2 결과 보고서: forward break-even pilots (FBP) Auto 경로의 사전 등록 평가")
    w("")
    w(f"- 노드: V27_ROUTER_HYPERPARAMETER_FREE (연구 루프 L1, wave 3, group 8)")
    w(f"- 브랜치 `research/production-loop-20261116-w3-v27`; phase 1 head `c998963d`; base(arm T) `1ffbda20`")
    w(f"- 사전 등록: `PREDECLARED.json` (sha256 `{PREDECLARED_SHA256[:16]}…`), `AMENDMENT_A8_2.json`, 정오표 A8.1·A8.3; 계산 방법: `PHASE2_NOTES.txt` (첫 결정 실행 전 커밋)")
    w(f"- 첫 결정 실행(UTC): {results['first_deciding_run_utc']}; 실행 {results['runs']['processes']}개 프로세스, 케이스 {results['runs']['cases']}개 x 팔(arm) 4개 x 3회")
    w(f"- 이 보고서와 RESULTS.json은 `parse_results.py`가 원시 runner 출력과 기준해(references)만 읽어 생성합니다. 벽시계 시간은 측정하지 않았습니다(카운터만).")
    w("")
    w("## 1. 결정")
    w("")
    verdicts = {k: v["verdict"] for k, v in d.items()}
    passed = all(v == "PASS" for v in verdicts.values())
    w(f"**동결된 결정 규칙**: {FROZEN_DECISION_TEXT}")
    w("")
    w(f"**결과**: {results['frozen_decision']['outcome_ko']}")
    w("")
    failing = [k for k, v in verdicts.items() if v != "PASS"]
    w(f"- 통과: {', '.join(k.split('_')[0] for k, v in verdicts.items() if v == 'PASS') or '-'}")
    w(f"- 실패: {', '.join(k.split('_')[0] for k in failing) or '-'}")
    w("")
    w("실패 원인과 결정에 쓰인 수:")
    w("")
    for cause in results["frozen_decision"]["causes_ko"]:
        w(f"- {cause}")
    w("")
    w("## 2. 무엇이 바뀌었나 (arm N)")
    w("")
    w("- 오늘의 Auto 경로(arm T)는 t0에서 강성 스크린(유한차분 열 1개)과 차원 한계(n > 10이면 RODAS5P), 1000회 DOPRI5 시험 예산을 썼습니다. 이 이름들과 규칙은 제거되었습니다(소유자 결정 D5).")
    w("- 새 경로(arm N, FBP): 선언이 없는 요청은 DOPRI5로 시작하고, DOPRI5 시도 수가 b·2^k에 이를 때마다 RODAS5P 한 번의 시도(pilot, h0 = H = DOPRI5가 마지막 b번 시도에서 전진한 길이, max_steps = 1)를 DOPRI5의 수락된 상태에서 시험합니다. pilot이 수락되면 RODAS5P로 넘어가고(handover, 단방향, D3), 거부되면 DOPRI5가 비트 단위로 그대로 계속합니다. b = ceil((2n^2 + 54n + 384)/156) 은 정수 산술로 n에서만 정해집니다.")
    w("- pilot 안의 치명적 콜백은 해를 CallbackFailed와 수락된 접두부로 끝냅니다(D1). 공급된 Jacobian/JVP/band는 RODAS5P로 시작합니다(D2). 선언 규칙은 그대로입니다.")
    w("")
    w("## 3. 판별자 E1-E10")
    w("")
    w("| 판별자 | 판정 | 결정에 쓰인 수 |")
    w("|---|---|---|")
    e1 = d["E1_economy"]
    w(f"| E1 경제성 | {e1['verdict']} | 케이스 {e1['cases']}개: 기하평균 N/T 콜백 {fmt(e1['geomean_cb_N_over_T'])}, 연산 {fmt(e1['geomean_op_N_over_T'])}; 최선 단일법 대비 최대비 콜백 N {fmt(e1['max_cb_N_over_best_single'])} ({e1['max_cb_N_over_best_single_case']}) vs T {fmt(e1['max_cb_T_over_best_single'])} ({e1['max_cb_T_over_best_single_case']}), 연산 N {fmt(e1['max_op_N_over_best_single'])} ({e1['max_op_N_over_best_single_case']}) vs T {fmt(e1['max_op_T_over_best_single'])} ({e1['max_op_T_over_best_single_case']}) |")
    e2 = d["E2_no_new_failure"]
    w(f"| E2 새 실패 없음 | {e2['verdict']} | T 성공 {e2['T_succeeds']}, N 성공 {e2['N_succeeds']}; T 성공·N 실패: {', '.join(e2['T_succeeds_N_not']) or '없음'}; N만 성공: {', '.join(e2['N_succeeds_T_not']) or '없음'}; 둘 다 실패: {', '.join(e2['both_fail']) or '없음'} |")
    e3 = d["E3_neutrality"]
    w(f"| E3 중립성 | {e3['verdict']} | (a) handover 없는 N 실행 {e3['a_runs_without_handover']}개 중 실패 {len(e3['a_failing'])}개: {', '.join(e3['a_failing']) or '없음'}; (b) 선언된 사실로 정해진 실행 {e3['b_runs_decided_by_a_declared_fact']}개 중 T와 다른 것: {', '.join(e3['b_failing']) or '없음'} |")
    e4 = d["E4_budget"]
    w(f"| E4 예산 | {e4['verdict']} | N 실행 {e4['runs']}개, 위반 {', '.join(e4['failing']) or '없음'}; 가장 빠듯한 경우 {', '.join(f'{x['case']} {x['attempts_all']}/{x['max_steps']}' for x in e4['tightest'])} |")
    e5 = d["E5_callbacks"]
    w(f"| E5 콜백 | {e5['verdict']} | N 실행 {e5['runs']}개(DOPRI5로 시작 {e5['runs_starting_with_dopri5']}개): 미과금 항목·재생 분해 실패 {', '.join(e5['failing']) or '없음'} |")
    e6 = d["E6_determinism"]
    w(f"| E6 결정성 | {e6['verdict']} | (케이스, 팔) {e6['case_arms']}개, 3회 실행 불일치 {', '.join(e6['failing']) or '없음'} |")
    e7 = d["E7_accuracy"]
    w(f"| E7 정확도 | {e7['verdict']} | 케이스 {e7['cases']}개, 위반 {', '.join(e7['failing']) or '없음'}; 최악 err(N)/max(err(D), err(R)) = {fmt(e7['worst_ratio'])} ({e7['worst_case']}) |")
    e8 = d["E8_callback_cases"]
    w(f"| E8 콜백 케이스 | {e8['verdict']} | F2 성공 {fmt(e8['checks']['F2_succeeds'])}; P_fatal CallbackFailed{{Rhs, 'sensor offline'}} {fmt(e8['checks']['P_fatal_callback_failed_rhs_sensor_offline'])}; N2 첫 pilot에서 CallbackFailed {fmt(e8['checks']['N2_callback_failed_at_first_pilot'])} |")
    e9 = d["E9_v03b_bounded_work"]
    e9a = {k: v for k, v in e9["per_case"].items() if v["decides"]}
    w(f"| E9 V03b 작업 한계 | {e9['verdict']} | " + "; ".join(f"{k}: 과금 rhs {v['charged_rhs']} ≤ {v['bound']}, handover {fmt(bool(v['handover']))}" for k, v in e9a.items()) + " |")
    e10 = d["E10_pilot_accounting"]
    w(f"| E10 pilot 회계 | {e10['verdict']} | pilot 있는 N 실행 {e10['runs_with_pilots']}개(없는 실행 {e10['runs_without_pilot']}개), 위반 {', '.join(e10['failing']) or '없음'}; pilot 지출/DOPRI5 지출 최대 콜백 {fmt(e10['max_spend_ratio_cb'])}, 연산 {fmt(e10['max_spend_ratio_op'])} |")
    w("")
    w(f"건전성(arms.sanity): 두 checkout으로 빌드한 D와 R의 비트 일치 {results['sanity']['compared']}건 비교, 불일치 {len(results['sanity']['disagreements'])}건 → {results['sanity']['verdict']}")
    w("")
    if e3["a_failing"]:
        w("### E3 (a) 실패 내역")
        w("")
        w("| 케이스 | 종료 방식 | 실패한 검사 | 콜백 N / D | 연산(x3) N / D |")
        w("|---|---|---|---|---|")
        for e in e3["a"]:
            if not e["holds"]:
                bad = [k for k, v in e.get("checks", {}).items() if not v]
                w(f"| {e['case']} | {e.get('ending', '-')} | {', '.join(bad) or e.get('why', '-')} | {e.get('cb_N', '-')} / {e.get('cb_D', '-')} | {e.get('op3_N', '-')} / {e.get('op3_D', '-')} |")
        w("")
        w(f"handover 없는 실행 중 치명적 pilot(D1)로 끝난 실행을 빼면 (a)가 모두 성립하는가(기술적 정보, 판정 기준 아님): {fmt(e3['a_holds_without_runs_ended_by_a_fatal_pilot_descriptive'])}")
        w("")
    w("## 4. 비용 표 (첫 번째 실행; 콜백 = 과금된 rhs + jvp + ft + Jacobian 콜백, 연산 = 사전 등록 공식; N/T 비는 둘 다 성공한 케이스만)")
    w("")
    w("| 케이스 | T 콜백 | N 콜백 | D 콜백 | R 콜백 | N/T 콜백 | T 연산 | N 연산 | D 연산 | R 연산 | N/T 연산 | N pilot (수락/손실/치명) | handover t (DOPRI5 시도) | 상태 N / T / D / R |")
    w("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for row in results["cases"]:
        a = row["arms"]

        def cb(arm):
            return fmt(a[arm]["cb"]) if a[arm]["cb"] is not None else "-"

        def op(arm):
            return fmt(a[arm]["operations"], 6) if a[arm].get("operations") is not None else "-"

        def short(arm):
            s = a[arm]["status"]
            return "ok" if s in ("success", "DOPRI5 reached the final time") else s.split(" {")[0][:28]

        both = a["N"]["success"] and a["T"]["success"]
        nt_cb = a["N"]["cb"] / a["T"]["cb"] if both else None
        nt_op = a["N"]["op3"] / a["T"]["op3"] if both else None
        ps = a["N"].get("pilot_summary") or {}
        pil = f"{ps.get('count', 0)} ({ps.get('accepted', 0)}/{ps.get('lost', 0)}/{ps.get('fatal', 0)})"
        ho = a["N"].get("handover")
        hot = f"{fmt(ho['time'])} ({ho['dopri5_attempts_before']})" if ho else "-"
        w(f"| {row['case']} | {cb('T')} | {cb('N')} | {cb('D')} | {cb('R')} | {fmt(nt_cb, 3)} | {op('T')} | {op('N')} | {op('D')} | {op('R')} | {fmt(nt_op, 3)} | {pil} | {hot} | {short('N')} / {short('T')} / {short('D')} / {short('R')} |")
    w("")
    w("## 5. 선언된 예상 손실과 측정값")
    w("")
    L = results["declared_expected_losses"]
    w(f"사전 등록 문구: {L['declared']}")
    w("")
    w("| 케이스 | 모델 N | 모델 T | 측정 N 콜백 | 측정 T 콜백 | 측정 N 연산(x3) | 측정 T 연산(x3) |")
    w("|---|---|---|---|---|---|---|")
    for x in L["named_cases"]:
        w(f"| {x['case']} | {x['declared_model_N_cb']} | {x['declared_model_T_cb']} | {x['measured_N_cb']} | {x['measured_T_cb']} | {x['measured_N_op3']} | {x['measured_T_op3']} |")
    w("")
    w("| 허용오차 | 비강성 케이스 | N이 T보다 콜백이 많은 케이스 | 콜백 증가 최소-최대 | 연산 증가 케이스 | 연산 증가 최대 |")
    w("|---|---|---|---|---|---|")
    for tol, x in L["non_stiff_by_tolerance"].items():
        pct = lambda v: "-" if v is None else f"{100 * v:+.1f}%"
        w(f"| {tol} | {x['cases']} | {x['cb_increase_cases']} | {pct(x['cb_increase_min'])} ~ {pct(x['cb_increase_max'])} | {x['op_increase_cases']} | {pct(x['op_increase_max'])} |")
    w("")
    w("## 6. 해석 규칙과 열린 문제")
    w("")
    for item in results["open_issues_ko"]:
        w(f"- {item}")
    w("")
    text = "\n".join(lines) + "\n"
    # case ids are problem|tolerance|options; a pipe would split a Markdown table cell
    return re.sub(r"([A-Za-z0-9_.\-]+)\|([ALS])\|(default|regime=[a-z]+|max_steps=[0-9]+)",
                  r"\1/\2/\3", text)


def main():
    rows, runs, meta, order, refs = load()
    cases, res = evaluate(runs, meta, order, refs)
    disc = {k: res[k] for k in ("E1_economy", "E2_no_new_failure", "E3_neutrality", "E4_budget",
                                "E5_callbacks", "E6_determinism", "E7_accuracy",
                                "E8_callback_cases", "E9_v03b_bounded_work", "E10_pilot_accounting")}
    for k in disc:
        disc[k]["text"] = DISCRIMINATOR_TEXT[k]
    all_pass = all(v["verdict"] == "PASS" for v in disc.values())
    failing = [k for k, v in disc.items() if v["verdict"] != "PASS"]
    outcome = ("adopt: the Auto route becomes N (FBP)" if all_pass else
               "keep today's route (T); the implementation stays on its branch and is not merged")
    refuted = []
    if any(k in failing for k in ("E3_neutrality", "E4_budget", "E5_callbacks", "E6_determinism", "E10_pilot_accounting")):
        refuted.append("the unconditional properties (design or implementation defect) by "
                       + ", ".join(k for k in failing if k.split("_")[0] in ("E3", "E4", "E5", "E6", "E10")))
    if "E1_economy" in failing:
        refuted.append("the economic claim (E1)")
    if "E2_no_new_failure" in failing:
        refuted.append("adoption blocked (E2)")
    causes_ko = []
    e1 = disc["E1_economy"]
    if e1["verdict"] == "FAIL":
        desc = e1["descriptive_break_even_route_only"]
        tied = desc["cases_where_the_maxima_of_N_and_T_are_attained_together"]
        bases = sorted({cases[c]["arms"]["N"].get("analysis_basis", {}).get("kind", "-") for c in tied})
        same_b = [e["case"] for e in disc["E3_neutrality"]["b"] if e["case"] in tied and e["holds"]]
        parts = e1["parts"]
        text = (f"E1: 기하평균 N/T는 콜백 {fmt(e1['geomean_cb_N_over_T'])}, 연산 {fmt(e1['geomean_op_N_over_T'])}"
                f" (1 미만 조건: 콜백 {fmt(parts['geomean_cb_below_1'])}, 연산 {fmt(parts['geomean_op_below_1'])}). "
                f"최선 단일법 대비 최대비 조건 N < T: 콜백 {fmt(parts['max_cb_N_below_T'])}, 연산 {fmt(parts['max_op_N_below_T'])}; "
                f"N의 최대 {fmt(e1['max_cb_N_over_best_single'])} ({e1['max_cb_N_over_best_single_case']}), "
                f"T의 최대 {fmt(e1['max_cb_T_over_best_single'])} ({e1['max_cb_T_over_best_single_case']}), "
                f"연산 N {fmt(e1['max_op_N_over_best_single'])} 대 T {fmt(e1['max_op_T_over_best_single'])}.")
        if tied:
            text += (f" 두 최대가 같은 케이스 {', '.join(tied)}에서 같은 값으로 나오므로 엄격한 부등식이 성립하지 않습니다. "
                     f"이 케이스의 분석 근거는 {', '.join(bases)}(선언된 사실)이며, N과 T는 E3 (b)에서 비트 단위로 같습니다"
                     f"({', '.join(same_b) or '-'}).")
        text += (f" 선언 없는 {desc['cases']}개 케이스만 보면(기술적 정보, 기준 아님) 최대비는 콜백 N "
                 f"{fmt(desc['max_cb_N_over_best_single']['value'])} ({desc['max_cb_N_over_best_single']['case']}) 대 T "
                 f"{fmt(desc['max_cb_T_over_best_single']['value'])} ({desc['max_cb_T_over_best_single']['case']}), 연산 N "
                 f"{fmt(desc['max_op_N_over_best_single']['value'])} ({desc['max_op_N_over_best_single']['case']}) 대 T "
                 f"{fmt(desc['max_op_T_over_best_single']['value'])} ({desc['max_op_T_over_best_single']['case']})입니다.")
        causes_ko.append(text)
    e3 = disc["E3_neutrality"]
    if e3["verdict"] == "FAIL":
        only_d1 = (not e3["b_failing"] and set(e3["a_failing_by_ending"]) == {"fatal pilot (D1)"})
        text = f"E3: (a) 실패 {', '.join(e3['a_failing']) or '없음'}, (b) 실패 {', '.join(e3['b_failing']) or '없음'}."
        if only_d1:
            text += (" (a)는 치명적 pilot(D1)으로 끝난 실행에서만 실패했습니다. D1과 E8은 N2가 첫 pilot에서 끝나기를 요구하고 "
                     "Dopri5::solve는 N2를 끝까지 풀기 때문에, 문자 그대로의 E3 (a)와 E8은 N2에서 동시에 성립할 수 없습니다(R1). "
                     "동결 규칙은 문자 그대로 적용했으며, 이 모순은 소유자 판단이 필요한 열린 문제입니다. "
                     "그 밖의 handover 없는 실행은 모두 (a)를 만족합니다.")
        causes_ko.append(text)
    e7 = disc["E7_accuracy"]
    if e7["verdict"] == "FAIL":
        for e in e7["per_case"]:
            if e["holds"]:
                continue
            ab = e["max_abs_error"]
            causes_ko.append(
                f"E7: {e['case']}: 가중 RMS 오차 err(N) = {fmt(e['err_N'])} > 3 x max(err(D), err(R)) = {fmt(e['bound'])} "
                f"(err(D) {fmt(e['err_D'])}, err(R) {fmt(e['err_R'])}, 비 {fmt(e['ratio_to_max'])}). 최대 절대오차 N {fmt(ab.get('N'))}, "
                f"R {fmt(ab.get('R'))}. 가중 노름에서 1이 허용오차 수준이므로 N의 오차는 허용오차 안이지만, 기준은 상대 비교이며 하한이 없습니다.")
    e5 = disc["E5_callbacks"]
    er = e5["erratum_E5_i"]
    if er["failing_under_first_noted_formula"]:
        causes_ko.append(
            f"E5 (정오 E5-i, 결정 실행 뒤 기록): 판정 {e5['verdict']}는 동결 문구 '미과금 항목 = 0'(과금되지 않은 콜백 항목)으로 계산했습니다. "
            f"PHASE2_NOTES 6에 처음 적은 공식(항목 - 과금 카운터 = 0, 모든 종류)으로는 {len(er['failing_under_first_noted_formula'])}개 실행"
            f"({', '.join(er['failing_under_first_noted_formula'])})이 실패하고 판정은 {er['verdict_under_first_noted_formula']}입니다. "
            "원인은 ft_calls가 시간미분 콜백이 없는 계산(유한차분 시간미분, P_mass)도 센다는 점이며, 같은 차이가 R과 T에도 있습니다"
            " (과금 카운터가 항목보다 큰 것이지 과금되지 않은 항목이 아닙니다). 동결 결정은 어느 계산이든 같습니다.")
    outcome_ko = ("채택: Auto 경로가 N(FBP)이 됩니다." if all_pass else
                  "유지: 오늘의 경로(T)가 유지되고, 구현은 브랜치에 남으며 병합되지 않습니다. "
                  + ("실패한 판별자: " + ", ".join(k.split("_")[0] for k in failing) + "."))
    first_utc = min(r["start_utc"] for r in rows)
    open_issues_ko = [
        "R1 (PHASE2_NOTES.txt 7): E3 (a)는 handover 없는 모든 N 실행에 문자 그대로 적용했습니다. 치명적 pilot으로 끝난 실행(D1)도 포함되므로, N2처럼 D1이 요구하는 종료는 D의 최종 상태 비트를 가질 수 없습니다. 이 경우를 따로 표시했지만 대체 기준으로 쓰지 않았습니다.",
        "R2: E5 (ii)/(iii)은 DOPRI5로 시작하는 실행에만 적용했습니다(선언된 규칙으로 RODAS5P로 시작하는 실행은 E3 (b)가 다룹니다). (i) 미과금 항목 0은 모든 실행에 적용했습니다.",
        "R3: E1의 최대비는 D 또는 R 중 하나라도 성공한 케이스에서만 계산했습니다. R4: E7은 N의 오차와 기준해가 있는 케이스에서만 계산했습니다. R5: 추가 허용오차 쌍은 기본 옵션의 해당 문제에만 적용했습니다.",
        "R6 (addendum 0b): E10은 pilot이 하나 이상인 N 실행에 적용했습니다(pilot이 없으면 셀 것이 없고, A_D = 0이면 공식이 정의되지 않습니다).",
        "E5 (iii)은 runner의 재생 분해(e5_split)로 판정했습니다: N의 콜백 항목이 비트 단위로 Dopri5::solve 항목의 접두부, 각 pilot의 재생(Dopri5::solve가 같은 시도 수 뒤 수락한 점에서 Rodas5p::solve, h0 = H, max_steps = 1), handover 뒤 RODAS5P 재생으로 정확히 나뉘어야 합니다. Solution은 dense output 없이 t0와 t1만 기록하므로 수락점 목록 대신 이 방법을 썼습니다.",
        "runner의 분류기는 비자율 문제에서 RODAS5P 시도의 첫 단계 f(t, y)를 옆의 유한차분 시간미분 호출과 같은 상태라는 이유로 시간미분으로 표시합니다. 종류별 수는 측정값으로만 보고하며 어떤 판정에도 쓰지 않았습니다.",
        "탐색용 팔 N_prod와 N_cont는 AMENDMENT_A8_2에 따라 실행하지 않았습니다.",
        "예제 crates/vigilode/examples/unified_production_solver.rs의 주석(모듈 문서 2·4항, 문제 4의 설명)은 break-even 경로를 설명하도록 고쳤습니다(코드 변경 없음). contract_4_fail_closed_protected_fallback은 과제가 적은 tests/unified_router_contracts.rs가 아니라 tests/unified_production_solver_contracts.rs(247-250행)에 있어 허용 경로 밖이므로 그 주석은 고치지 않았습니다. tests/unified_router_contracts.rs 1864-1865행의 주석도 제거된 스크린과 시험 예산을 설명하지만 허용 범위(contract_4의 주석) 밖이라 그대로 두었습니다.",
        "E1의 최대비 조건은 엄격한 부등식이고 사례 집합에 선언된 사실의 케이스가 들어 있어, 두 경로가 같은 경로를 실행하는 케이스가 두 최대를 함께 정하면 조건이 성립할 수 없습니다. 이번에는 Robertson에 Nonstiff를 선언한 케이스가 그렇습니다. 사전 등록 문구 그대로 적용했고, 이 구조는 소유자 판단이 필요한 열린 문제입니다.",
        "E7은 상대 비교(err(N) <= 3 max(err(D), err(R)))이며 절대 하한이 없습니다. 실패한 케이스에서 N의 가중 오차는 허용오차 수준(가중 노름 1)보다 훨씬 작지만 R의 3배를 넘습니다(수치는 1절과 RESULTS.json의 E7 항목). 원인은 이번 평가에서 분석하지 않았습니다(가설: handover 뒤 RODAS5P가 h0 = H로 시작해 R과 다른 단계 순서를 밟음). 문구 그대로 실패로 판정했습니다.",
    ]
    results = {
        "schema": "vigilode.v27.phase2.results.v1",
        "node": "V27_ROUTER_HYPERPARAMETER_FREE",
        "predeclared_sha256": PREDECLARED_SHA256,
        "first_deciding_run_utc": first_utc,
        "runs": {"processes": len(rows), "cases": len(cases),
                 "lines": sum(len(v) for v in runs.values())},
        "sanity": res["sanity"],
        "discriminators": disc,
        "frozen_decision": {"rule": FROZEN_DECISION_TEXT, "all_pass": all_pass,
                            "failing": failing, "outcome": outcome, "refutes": refuted,
                            "outcome_ko": outcome_ko, "causes_ko": causes_ko},
        "declared_expected_losses": losses(cases),
        "cases": case_table(cases),
        "open_issues_ko": open_issues_ko,
    }
    text = json.dumps(results, indent=1, sort_keys=True, allow_nan=False, ensure_ascii=False)
    with open(OUT_JSON, "w") as handle:
        handle.write(text + "\n")
    with open(OUT_MD, "w") as handle:
        handle.write(report(results))
    print(json.dumps({"decision": outcome, "verdicts": {k: v["verdict"] for k, v in disc.items()},
                      "sanity": res["sanity"]["verdict"], "first_deciding_run_utc": first_utc}))


if __name__ == "__main__":
    sys.exit(main())
