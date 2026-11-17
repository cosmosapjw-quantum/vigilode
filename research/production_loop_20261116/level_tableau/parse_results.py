#!/usr/bin/env python3
"""Loop L1 node V16 -- build RESULTS.json from the raw logs only (no typed-in numbers).

    python3 -I parse_results.py            # writes RESULTS.json next to this file, prints a short summary

Inputs: logs/*.jsonl (JSON lines written by the run scripts), PREDECLARED.json (protocol text only),
../cost_anatomy/AMDAHL.json and REVIEW_OVERLAY.json (chain shares for the speed-up model).
Rules implemented here are the ones of PREDECLARED.json: matched-error rule, selection rule, observed order,
verdict thresholds of the frozen row (addendum A1 of the pre-registration).
"""
import glob
import hashlib
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LOGS = os.path.join(HERE, "logs")
COST = os.path.join(HERE, "..", "cost_anatomy")

# ---- protocol constants (PREDECLARED.json and the frozen row)
FROZEN = {"tool_validation_tol": 1e-12, "order_min": 4.8,
          "depth5": {"pass": 1.25, "gain": 1.6}, "depth6": {"pass": 1.10, "gain": 8.0 / 6.0}}
KILL_LEVEL = {"depth5": 1.6, "depth6": 1.33}
ORDER_FLOOR = 1e-12           # amendment A-2
ORDER_MIN_PAIRS = 3
TARGETS = [10.0 ** (-m / 2) for m in range(6, 19)]
MIN_TARGETS = 3
SCREEN_PROBLEMS = ["ROBER", "VDPOL", "BURGERS"]
DEV_PROBLEMS = ["HIRES", "OREGO"]
ORDER_PROBLEMS = ["BRUSS_ODE", "VDP_MU2", "PENDULUM_FORCED"]
LAYOUT_OF = {"D5F": "depth5", "D5G": "depth5", "D6T": "depth6", "D6L": "depth6", "D6F": "depth6"}
DEPTH = {"depth5": 5, "depth6": 6}


def rows(path):
    out = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        h.update(fh.read())
    return h.hexdigest()


def logs(pattern):
    return sorted(p for p in glob.glob(os.path.join(LOGS, pattern)) if not os.path.basename(p).startswith("smoke"))


def quantiles(v):
    v = sorted(v)
    if not v:
        return None
    pick = lambda q: v[min(len(v) - 1, int(q * (len(v) - 1) + 0.5))]   # noqa: E731
    return {"min": v[0], "q10": pick(0.1), "median": pick(0.5), "q90": pick(0.9), "max": v[-1], "count": len(v)}


# ------------------------------------------------------------------------------------------- matched error
def pareto_front(points):
    pts = sorted(set(points))
    front = [p for p in pts if not any(q[0] <= p[0] and q[1] <= p[1] and (q[0] < p[0] or q[1] < p[1]) for q in pts)]
    return sorted(front)


def attempts_at(front, E):
    if len(front) < 2 or E < front[0][0] or E > front[-1][0]:
        return None
    for (e0, a0), (e1, a1) in zip(front[:-1], front[1:]):
        if e0 <= E <= e1:
            if e1 == e0:
                return float(min(a0, a1))
            w = (math.log(E) - math.log(e0)) / (math.log(e1) - math.log(e0))
            return math.exp(math.log(a0) + w * (math.log(a1) - math.log(a0)))
    return None


def matched_ratio(new_pts, ref_pts, u_ref):
    fn, fr = pareto_front(new_pts), pareto_front(ref_pts)
    used = []
    for E in TARGETS:
        if u_ref is None or E < 100.0 * u_ref:
            continue
        an, ar = attempts_at(fn, E), attempts_at(fr, E)
        if an is not None and ar is not None:
            used.append({"target_error": E, "attempts_new": an, "attempts_rodas5p": ar, "ratio": an / ar})
    out = {"front_new": fn, "front_rodas5p": fr, "usable_targets": len(used), "targets": used}
    if len(used) >= MIN_TARGETS:
        out["ratio"] = math.exp(sum(math.log(u["ratio"]) for u in used) / len(used))
        out["ratio_min"] = min(u["ratio"] for u in used)
        out["ratio_max"] = max(u["ratio"] for u in used)
        out["status"] = "DECIDED"
    else:
        out["ratio"] = None
        out["status"] = "NOT_DECIDABLE"
    return out


def collect_payoff(label):
    """{problem: {"u_ref": .., "arms": {arm: [(error, attempts)]}, "runs": {arm: [...]}}} for one label."""
    data = {}
    files = []
    for path in logs("payoff_%s*.jsonl" % label):
        files.append(os.path.basename(path))
        for r in rows(path):
            if r.get("label") != label:
                continue
            if r["kind"] == "reference":
                d = data.setdefault(r["problem"], {"u_ref": None, "arms": {}, "runs": {}, "references": []})
                d["references"].append({k: r.get(k) for k in ("u_ref", "diff_radau_rtol_1em11",
                                                               "diff_rodas5p_prototype_rtol_1em11",
                                                               "diff_lsoda_rtol_1em12_reported_only", "n", "t_end",
                                                               "a_scale", "method")})
                if r.get("u_ref") is not None:
                    d["u_ref"] = max(d["u_ref"] or 0.0, r["u_ref"])
            elif r["kind"] == "run":
                d = data.setdefault(r["problem"], {"u_ref": None, "arms": {}, "runs": {}, "references": []})
                key = (r["arm"], r["k"])
                rec = {"k": r["k"], "rtol": r["rtol"], "finished": r["finished"], "accepted": r["accepted"],
                       "rejected": r["rejected"], "attempts": r["attempts"], "error": r.get("error")}
                prev = d["runs"].setdefault(r["arm"], {})
                if r["k"] in prev and prev[r["k"]] != rec:
                    rec["inconsistent_with_other_log"] = True
                prev[r["k"]] = rec
                _ = key
    for pid, d in data.items():
        for arm, byk in d["runs"].items():
            d["arms"][arm] = [(v["error"], v["attempts"]) for v in byk.values()
                              if v["finished"] and v["error"] is not None and v["error"] > 0]
            d["runs"][arm] = [byk[k] for k in sorted(byk)]
    return data, files


def main():
    res = {"schema": "vigilode.loop_l1.v16_level_tableau.results.v1", "node": "V16_LEVEL_TABLEAU_SEARCH",
           "generated_by": "parse_results.py from logs/*.jsonl", "frozen_thresholds": FROZEN,
           "kill_level_as_printed_in_the_frozen_row": KILL_LEVEL,
           "protocol": {"order_floor": ORDER_FLOOR, "order_min_pairs": ORDER_MIN_PAIRS, "targets": TARGETS,
                        "min_targets": MIN_TARGETS}}
    inputs = {}
    for p in sorted(glob.glob(os.path.join(LOGS, "*.jsonl"))):
        inputs[os.path.relpath(p, HERE)] = sha(p)
    res["inputs_sha256"] = inputs
    pre = json.load(open(os.path.join(HERE, "PREDECLARED.json")))
    res["amendments"] = [{"id": a["id"], "scope": a["scope"]} for a in pre.get("amendments", [])]

    # ------------------------------------------------------------------ tool validation
    val = rows(os.path.join(LOGS, "validation.jsonl"))
    fz = [r for r in val if r["kind"] == "frozen_tool_validation"][0]
    i1 = [r for r in val if r["kind"] == "index1_summary"][0]
    ec = [r for r in val if r["kind"] == "error_coefficients"][0]
    res["tool_validation"] = {
        "conditions": fz["conditions"], "max_abs_residual_binary64": fz["max_abs_residual_binary64"],
        "max_abs_residual_mp60": fz["max_abs_residual_mp60"], "threshold": FROZEN["tool_validation_tol"],
        "passed": bool(fz["max_abs_residual_binary64"] <= FROZEN["tool_validation_tol"]),
        "tree_counts": [r for r in val if r["kind"] == "tree_counts"][0]["counts"],
        "index1_generated": i1["generated"], "index1_selected_main": i1["selected_main"],
        "index1_selected_embedded": i1["selected_embedded"],
        "index1_not_vanishing_on_rodas5p": [{"tree": r["tree"], "order": r["order"], "root": r["root"],
                                              "residual_b": r["residual_b"]}
                                             for r in val if r["kind"] == "index1_condition" and not r["selected_main"]],
        "rodas5p_e6_norm": ec["e6_norm"], "rodas5p_ehat5_norm": ec["ehat5_norm"],
        "depth5_gamma": [r for r in val if r["kind"] == "depth5_gamma"][0]["gamma"],
        "depth5_embedded_obstruction_value": [r for r in val if r["kind"] == "depth5_embedded_obstruction"][0][
            "value_at_pinned_gamma"]}

    # ------------------------------------------------------------------ search
    search = {}
    for path in logs("search_*.jsonl"):
        fid = os.path.basename(path)[len("search_"):-len(".jsonl")]
        rs = rows(path)
        st = [r for r in rs if r["kind"] == "start"]
        hd = [r for r in rs if r["kind"] == "header"]
        ft = [r for r in rs if r["kind"] == "footer"]
        ok = [r for r in st if r.get("pass_double")]
        entry = {"layout": LAYOUT_OF.get(fid), "starts": len(st), "complete": bool(ft),
                 "start_indices": [min(r["k"] for r in st), max(r["k"] for r in st)] if st else None,
                 "unknowns": hd[0]["unknowns"] if hd else None,
                 "hard_conditions": hd[0]["hard_conditions"] if hd else None,
                 "passed_double": len(ok), "failed_exceptions": sum(1 for r in st if "failed" in r),
                 "gamma_out_of_window": sum(1 for r in st if r.get("gamma_in_window") is False),
                 "score_of_passing": quantiles([r["score"] for r in ok if "score" in r]),
                 "max_hard_of_passing": quantiles([r["max_hard"] for r in ok]),
                 "seconds": ft[0]["seconds"] if ft else None}
        pp = os.path.join(LOGS, "polish_%s.jsonl" % fid)
        if os.path.exists(pp):
            pr = [r for r in rows(pp) if r["kind"] == "polish"]
            okp = [r for r in pr if r.get("pass_double")]
            entry["phase_b"] = {"polished": len(pr), "passed_double": len(okp),
                                "score": quantiles([r["score"] for r in okp if "score" in r]),
                                "gamma": quantiles([r["gamma"] for r in okp]),
                                "coef_max": quantiles([r["coef_max"] for r in okp])}
        cp = os.path.join(LOGS, "continuation_%s.jsonl" % fid)
        if os.path.exists(cp):
            cr = [r for r in rows(cp) if r["kind"] == "continuation"]
            okc = [r for r in cr if r.get("pass_double")]
            best = {}
            for r in okc:
                g = r["gamma_fixed"]
                if "score" in r and (g not in best or r["score"] < best[g]):
                    best[g] = r["score"]
            entry["continuation"] = {"solves": len(cr), "passed_double": len(okc),
                                     "best_score_by_gamma": [{"gamma": g, "score": best[g]} for g in sorted(best)]}
        search[fid] = entry
    res["search"] = search

    # ------------------------------------------------------------------ refinement and verification
    cands = {}
    for path in logs("refine_*.jsonl"):
        for r in rows(path):
            if r["kind"] == "refined" and not r["id"].startswith("smoke"):
                cands[r["id"]] = {"formulation": r["fid"], "layout": LAYOUT_OF.get(r["fid"]), "rank_by_score": r["rank"],
                                  "file": r["file"], "score_double": r["source"]["score_double"],
                                  "parts_double": r["source"]["parts_double"], "source": r["source"],
                                  "refine_iterations": r["iterations"], "refine_max_hard_mp": r["max_hard_mp"]}
    for path in logs("verify_*.jsonl"):
        for r in rows(path):
            if r["kind"] == "verify" and r["id"] in cands:
                keep = ("gamma", "depth", "stages", "levels", "min_significant_digits",
                        "order5_max_abs_residual_exact", "order5_max_abs_residual_binary64",
                        "stiffly_accurate_exact", "R_infinity_exact", "numerator_minus_design_max",
                        "E_sturm_roots_in_positive_s", "E_leading_coefficient_positive", "grid_max_abs_R_minus_1",
                        "A_stable", "embedded_order4_max_abs_residual_exact",
                        "embedded_order4_max_abs_residual_binary64", "embedded_order5_max_abs_residual",
                        "embedded_R_infinity", "embedded_grid_max_abs_R_minus_1", "embedded_A_stable_on_grid",
                        "W_sum_b_gamma_i_exact", "W_sum_b_gamma_i_binary64", "max_abs_coefficient", "c",
                        "stability_taylor_defects_z1_to_z7", "checks", "algebraic_pass")
                cands[r["id"]]["verify"] = {k: r[k] for k in keep}
                cands[r["id"]]["verify_log"] = os.path.basename(path)
    res["candidates"] = cands

    # ------------------------------------------------------------------ development screen and selection
    dev, dev_files = collect_payoff("dev")
    selection = {}
    for cid, c in cands.items():
        if not c.get("verify", {}).get("algebraic_pass"):
            continue
        ratios = {}
        for pid in DEV_PROBLEMS:
            d = dev.get(pid)
            if d and cid in d["arms"] and "RODAS5P" in d["arms"]:
                m = matched_ratio(d["arms"][cid], d["arms"]["RODAS5P"], d["u_ref"])
                failed = any(not r["finished"] for r in d["runs"][cid])
                ratios[pid] = {"ratio": m["ratio"], "usable_targets": m["usable_targets"], "status": m["status"],
                               "failed_runs": int(sum(1 for r in d["runs"][cid] if not r["finished"])),
                               "counted_as": (m["ratio"] if (m["ratio"] is not None and not failed) else 10.0)}
        if len(ratios) == len(DEV_PROBLEMS):
            c["development"] = ratios
            c["development_geomean"] = math.exp(sum(math.log(v["counted_as"]) for v in ratios.values()) / len(ratios))
    for layout in ("depth5", "depth6"):
        pool = [(c.get("development_geomean"), c["score_double"], cid) for cid, c in cands.items()
                if c["layout"] == layout and c.get("verify", {}).get("algebraic_pass")]
        with_dev = sorted(p for p in pool if p[0] is not None)
        entry = {"algebraic_pass_candidates": sorted(p[2] for p in pool), "development_logs": dev_files}
        if with_dev:
            entry["primary"] = with_dev[0][2]
            entry["rule"] = "lowest geometric mean of the development attempt ratios"
            entry["ranking"] = [{"id": p[2], "development_geomean": p[0], "score": p[1]} for p in with_dev]
        elif pool:
            entry["primary"] = sorted(pool, key=lambda p: p[1])[0][2]
            entry["rule"] = "lowest score (no development ratio available)"
        else:
            entry["primary"] = None
        selection[layout] = entry
    res["selection"] = selection
    res["development_references"] = {pid: {"u_ref": d["u_ref"], "references": d["references"]} for pid, d in dev.items()}

    # ------------------------------------------------------------------ order check
    order = {}
    pr_runs = {}
    for path in logs("order_*.jsonl"):
        for r in rows(path):
            if r["kind"] == "order_point":
                order.setdefault(r["arm"], {}).setdefault(r["problem"], {})[r["steps"]] = r["error"]
            elif r["kind"] == "pr_point":
                pr_runs.setdefault(r["arm"], {}).setdefault(str(r.get("lam")), {})[r["steps"]] = r["error"]
            elif r["kind"] == "order_reference":
                order.setdefault("_references", {})[r["problem"]] = {"uncertainty_vs_dop853": r["uncertainty_vs_dop853"],
                                                                      "y_ref": r["y_ref"]}
    order_out = {"references": order.pop("_references", {})}
    for arm, probs in order.items():
        a = {}
        for pid in ORDER_PROBLEMS:
            if pid not in probs:
                continue
            steps = sorted(probs[pid])
            errs = [probs[pid][n] for n in steps]
            pairs = []
            for i in range(len(steps) - 1):
                e0, e1 = errs[i], errs[i + 1]
                if e0 is not None and e1 is not None and e0 >= ORDER_FLOOR and e1 >= ORDER_FLOOR and steps[i + 1] == 2 * steps[i]:
                    pairs.append({"steps": [steps[i], steps[i + 1]], "order": math.log2(e0 / e1)})
            entry = {"steps": steps, "errors": errs, "pairs": pairs}
            if len(pairs) >= ORDER_MIN_PAIRS:
                v = sorted(p["order"] for p in pairs)
                med = v[len(v) // 2] if len(v) % 2 else 0.5 * (v[len(v) // 2 - 1] + v[len(v) // 2])
                entry["observed_order"] = med
                entry["status"] = "PASS" if med >= FROZEN["order_min"] else "FAIL"
            else:
                entry["observed_order"] = None
                entry["status"] = "NOT_DECIDABLE"
            a[pid] = entry
        order_out[arm] = a
    res["order_check"] = order_out
    res["prothero_robinson_exploratory"] = {
        arm: {lam: {"steps": sorted(v), "errors": [v[n] for n in sorted(v)],
                    "pair_orders": [(math.log2(v[a] / v[b]) if v[a] and v[b] else None)
                                    for a, b in zip(sorted(v)[:-1], sorted(v)[1:])]}
              for lam, v in d.items()} for arm, d in pr_runs.items()}

    # ------------------------------------------------------------------ pay-off screen
    scr, scr_files = collect_payoff("screen")
    payoff = {"logs": scr_files, "references": {pid: {"u_ref": d["u_ref"], "references": d["references"]}
                                                 for pid, d in scr.items()}, "arms": {}}
    arms = sorted({arm for d in scr.values() for arm in d["arms"] if arm != "RODAS5P"})
    for arm in arms:
        a = {}
        for pid in SCREEN_PROBLEMS:
            d = scr.get(pid)
            if d and arm in d["arms"] and "RODAS5P" in d["arms"]:
                m = matched_ratio(d["arms"][arm], d["arms"]["RODAS5P"], d["u_ref"])
                m["runs_new"] = d["runs"][arm]
                m["runs_rodas5p"] = d["runs"]["RODAS5P"]
                m["failed_runs_new"] = int(sum(1 for r in d["runs"][arm] if not r["finished"]))
                a[pid] = m
        payoff["arms"][arm] = a
    res["payoff_screen"] = payoff

    # ------------------------------------------------------------------ verdicts
    verdicts = {}
    for layout in ("depth5", "depth6"):
        sel = selection[layout]
        prim = sel.get("primary")
        v = {"primary_candidate": prim, "found_algebraic": bool(sel["algebraic_pass_candidates"]),
             "formulations_run": sorted(f for f, e in search.items() if e["layout"] == layout),
             "starts_total": sum(e["starts"] for f, e in search.items() if e["layout"] == layout)}
        if not res["tool_validation"]["passed"]:
            v["verdict"] = "BLOCKED_TOOL_VALIDATION"
        elif not v["found_algebraic"]:
            v["verdict"] = "NOT_FOUND_WITHIN_BUDGET"
        else:
            oc = order_out.get(prim, {})
            st = [oc.get(p, {}).get("status", "NOT_RUN") for p in ORDER_PROBLEMS]
            v["order_status"] = dict(zip(ORDER_PROBLEMS, st))
            v["observed_orders"] = {p: oc.get(p, {}).get("observed_order") for p in ORDER_PROBLEMS}
            v["order_ok"] = all(s == "PASS" for s in st)
            pa = payoff["arms"].get(prim, {})
            ratios = {p: pa.get(p, {}).get("ratio") for p in SCREEN_PROBLEMS}
            v["attempt_ratios"] = ratios
            v["usable_targets"] = {p: pa.get(p, {}).get("usable_targets") for p in SCREEN_PROBLEMS}
            thr, gain = FROZEN[layout]["pass"], KILL_LEVEL[layout]
            dec = [r for r in ratios.values() if r is not None]
            v["problems_at_or_below_pass_threshold"] = sum(1 for r in dec if r <= thr)
            v["problems_at_or_above_depth_gain"] = sum(1 for r in dec if r >= gain)
            v["pass_threshold"], v["depth_gain_kill_level"] = thr, gain
            if not pa:
                v["payoff"] = "NOT_RUN"
            elif v["problems_at_or_below_pass_threshold"] >= 2:
                v["payoff"] = "PASS"
            elif len(dec) == 3 and v["problems_at_or_above_depth_gain"] == 3:
                v["payoff"] = "KILL"
            elif len(dec) < 3 and v["problems_at_or_below_pass_threshold"] + (3 - len(dec)) >= 2:
                v["payoff"] = "NOT_DECIDABLE"
            else:
                v["payoff"] = "FAIL"
            if v["payoff"] == "PASS" and v["order_ok"]:
                v["verdict"] = "FOUND_PASS"
            elif v["payoff"] == "KILL":
                v["verdict"] = "FOUND_KILL"
            elif v["payoff"] in ("NOT_RUN", "NOT_DECIDABLE") or not all(s in ("PASS", "FAIL") for s in st):
                v["verdict"] = "FOUND_NOT_DECIDABLE"
            else:
                v["verdict"] = "FOUND_FAIL"
        verdicts[layout] = v
    vs = [verdicts[k]["verdict"] for k in ("depth5", "depth6")]
    if any(x == "FOUND_PASS" for x in vs):
        node = "PASS"
    elif all(x == "NOT_FOUND_WITHIN_BUDGET" for x in vs) or (
            any(x == "FOUND_KILL" for x in vs) and all(x in ("FOUND_KILL", "NOT_FOUND_WITHIN_BUDGET") for x in vs)):
        node = "KILL"
    elif any(x in ("FOUND_NOT_DECIDABLE", "BLOCKED_TOOL_VALIDATION") for x in vs):
        node = "NOT_DECIDABLE"
    else:
        node = "FAIL"
    verdicts["node"] = node
    verdicts["node_rule"] = ("PASS: a layout is found, has observed order >= 4.8 and passes its pay-off threshold on 2 of 3 "
                             "problems. KILL (frozen row): no layout found within the budget, or every found layout has "
                             "an attempt ratio >= its depth gain on all three problems. FAIL: found, no layout passes, "
                             "and not every found layout is at the kill level.")
    res["verdicts"] = verdicts

    # ------------------------------------------------------------------ exploratory
    expl = {"label": "exploratory", "methods": {}, "linear_bound_layout": []}
    for path in logs("exploratory*.jsonl"):
        for r in rows(path):
            k = r["kind"]
            if k in ("error_terms", "stability", "code_form", "schedule"):
                m = expl["methods"].setdefault(r["method"], {})
                m[k] = {a: b for a, b in r.items() if a not in ("kind", "method", "label")}
            elif k == "linear_bound_reference":
                expl["linear_bound_rodas5p_reach"] = r["linear_reach_abs_z"]
            elif k == "linear_bound_layout":
                expl["linear_bound_layout"].append({a: b for a, b in r.items() if a not in ("kind", "label")})
            elif k == "header":
                expl["eps"] = r["eps"]
    # relative size of the leading error terms
    rod = expl["methods"].get("RODAS5P", {}).get("error_terms")
    if rod:
        for name, m in expl["methods"].items():
            if "error_terms" in m:
                m["relative_to_rodas5p"] = {"e6_norm": m["error_terms"]["e6_norm"] / rod["e6_norm"],
                                            "ehat5_norm": m["error_terms"]["ehat5_norm"] / rod["ehat5_norm"]}
    # modelled speed-up of the whole step (model, not a measurement)
    am_path = os.path.join(COST, "AMDAHL.json")
    ov_path = os.path.join(COST, "REVIEW_OVERLAY.json")
    if os.path.exists(am_path):
        am = json.load(open(am_path))
        overlay = {}
        if os.path.exists(ov_path):
            overlay = {r["cell"]: r["overlay_label"] for r in json.load(open(ov_path)).get("rows", [])}
        model = {"formula": "S = 1 / ((1 - f_chain + f_chain * L / 8) * attempt_ratio)",
                 "source": "cost_anatomy/AMDAHL.json (f_chain per regime), REVIEW_OVERLAY.json (labels)",
                 "regimes": len(am["regimes"]), "layouts": {}}
        for layout in ("depth5", "depth6"):
            L = DEPTH[layout]
            ratios = verdicts[layout].get("attempt_ratios") or {}
            per = {}
            for pid, r in ratios.items():
                if r is None:
                    continue
                vals_all, vals_auth = [], []
                for reg in am["regimes"]:
                    f = reg["fractions"]["f_chain"]
                    s_ = 1.0 / ((1.0 - f + f * L / 8.0) * r)
                    vals_all.append(s_)
                    if overlay.get(reg["cell"], reg.get("admissibility")) == "authoritative":
                        vals_auth.append(s_)
                per[pid] = {"attempt_ratio": r, "chain_only_bound": (8.0 / L) / r, "all_regimes": quantiles(vals_all),
                            "overlay_authoritative_regimes": quantiles(vals_auth),
                            "regimes_above_1_all": sum(1 for x in vals_all if x > 1.0),
                            "regimes_above_1_authoritative": sum(1 for x in vals_auth if x > 1.0)}
            model["layouts"][layout] = per
        expl["modelled_step_speedup"] = model
    res["exploratory"] = expl

    out = os.path.join(HERE, "RESULTS.json")
    with open(out, "w") as fh:
        json.dump(res, fh, indent=1, allow_nan=False)
        fh.write("\n")
    # short summary
    print("tool validation:", res["tool_validation"]["passed"], res["tool_validation"]["max_abs_residual_binary64"])
    for fid, e in search.items():
        print("search", fid, "starts", e["starts"], "passed", e["passed_double"], "complete", e["complete"])
    for layout in ("depth5", "depth6"):
        v = verdicts[layout]
        print(layout, v["verdict"], "primary", v.get("primary_candidate"), "orders", v.get("observed_orders"),
              "ratios", v.get("attempt_ratios"), "payoff", v.get("payoff"))
    print("node:", node)
    return 0


if __name__ == "__main__":
    sys.exit(main())
