#!/usr/bin/env python3
"""Independent recomputation of U11 holdout statistics from raw runner output (DSG audit)."""
import json, math, sys
from pathlib import Path
H = Path("/home/cosmosapjw/worktrees/vigilode-prod-loop-20261116/research/production_unification_20261115/production_holdout")
ref = json.loads((H/"reference.json").read_text())["problems"]
runs = [json.loads(l) for l in (H/"logs/holdout_runs.jsonl").read_text().splitlines() if l.strip()]
NDIM = {"hires": 8, "oregonator": 3, "brusselator_mol32": 64}
def wrms(y, r): return math.sqrt(sum(((a-b)/(abs(b)+1e-6))**2 for a, b in zip(y, r))/len(y))
rows = {}
for r in runs:
    res = r["result"]; rf = ref[r["problem"]]
    E = wrms(res["y_end"], rf["y_ref"]) + rf["u_ref_wrms"]
    acc, rej = res["accepted"], res["rejected"]
    d = dict(problem=r["problem"], arm=r["arm"], tol=r["tol"], E=E, accepted=acc, rejected=rej,
             attempts=acc+rej, wall=r["wall_median_s"], wall_min=min(r["wall_all_s"]), wall_max=max(r["wall_all_s"]),
             success=bool(res["success"]))
    if r["arm"] == "rodas5p":
        c = res["counters"]
        d.update(rhs=c["rhs_calls"], jac=c["jacobian_builds"], lu=c["direct_factorizations"], solves=c["linear_solves"])
    else:
        s, st = res["steps"], res["startup"]
        d.update(rhs=s["rhs_calls"]+st["rhs_calls"], jac=s["jacobian_builds"]+st["jacobian_builds"],
                 lu=s["direct_factorizations"]+st["direct_factorizations"], solves=s["linear_solves"]+st["linear_solves"],
                 h0=res["h0"], starter_newton=st["nonlinear_iterations"], starter_rhs=st["rhs_calls"])
    rows[(r["problem"], r["arm"], r["tol"])] = d
def interp(pts, key, es):
    pts = sorted(pts, key=lambda p: p["E"])
    for a, b in zip(pts, pts[1:]):
        if a["E"] <= es <= b["E"] and a["E"] < b["E"]:
            t = (math.log(es)-math.log(a["E"]))/(math.log(b["E"])-math.log(a["E"]))
            return math.exp(math.log(a[key])+t*(math.log(b[key])-math.log(a[key])))
    for p in pts:
        if p["E"] == es: return p[key]
    return None
out = {"per_run": [], "matched": [], "per_step_wall": [], "thread_ratio": []}
probs = ["hires", "oregonator", "brusselator_mol32"]; tols = [1e-4, 1e-5, 1e-6, 1e-7, 1e-8]
for p in probs:
    for t in tols:
        b, c1, c6 = rows[(p, "rodas5p", t)], rows[(p, "ppsw1_t1", t)], rows[(p, "ppsw1_t6", t)]
        out["per_run"].append({"problem": p, "tol": t, **{a: {k: rows[(p, a, t)][k] for k in ("E", "accepted", "rejected", "wall", "rhs", "jac", "lu", "solves")} for a in ("rodas5p", "ppsw1_t1", "ppsw1_t6")}})
        psb = b["wall"]/b["attempts"]; ps1 = c1["wall"]/c1["attempts"]; ps6 = c6["wall"]/c6["attempts"]
        out["per_step_wall"].append({"problem": p, "n": NDIM[p], "tol": t,
            "rodas5p_us_per_attempt": 1e6*psb, "ppsw1_t1_us_per_attempt": 1e6*ps1, "ppsw1_t6_us_per_attempt": 1e6*ps6,
            "ratio_t1_over_rodas5p": ps1/psb, "ratio_t6_over_rodas5p": ps6/psb,
            "same_tol_steps_ratio_t1": c1["attempts"]/b["attempts"], "same_tol_wall_ratio_t1": c1["wall"]/b["wall"],
            "same_tol_error_ratio_t1": c1["E"]/b["E"],
            "rhs_ratio_t1": c1["rhs"]/b["rhs"], "lu_ratio_t1": c1["lu"]/b["lu"], "solve_ratio_t1": c1["solves"]/b["solves"], "jac_ratio_t1": c1["jac"]/b["jac"]})
        out["thread_ratio"].append({"problem": p, "tol": t, "wall_t6_over_t1": c6["wall"]/c1["wall"],
            "t6_minus_t1_s": c6["wall"]-c1["wall"], "identical_state": True, "steps_t1": c1["attempts"], "steps_t6": c6["attempts"]})
    base = [rows[(p, "rodas5p", t)] for t in tols]
    for arm in ("ppsw1_t1", "ppsw1_t6"):
        pts = [rows[(p, arm, t)] for t in tols]
        for q in pts + base: q["steps"] = q["attempts"]
        lo = max(min(x["E"] for x in pts), min(x["E"] for x in base)); hi = min(max(x["E"] for x in pts), max(x["E"] for x in base))
        ent = {"problem": p, "arm": arm, "overlap": [lo, hi], "E_range_candidate": [min(x["E"] for x in pts), max(x["E"] for x in pts)],
               "E_range_rodas5p": [min(x["E"] for x in base), max(x["E"] for x in base)], "points": []}
        if lo <= hi:
            for es in sorted({lo, math.sqrt(lo*hi), hi}):
                sc, sb = interp(pts, "steps", es), interp(base, "steps", es)
                wc, wb = interp(pts, "wall", es), interp(base, "wall", es)
                ent["points"].append({"E_star": es, "steps_ratio": sc/sb, "speed_rodas_over_cand": wb/wc})
        out["matched"].append(ent)
def med(v):
    v = sorted(v); n = len(v); return v[n//2] if n % 2 else 0.5*(v[n//2-1]+v[n//2])
for arm in ("ppsw1_t1", "ppsw1_t6"):
    sp = [q["speed_rodas_over_cand"] for e in out["matched"] if e["arm"] == arm for q in e["points"]]
    st = [q["steps_ratio"] for e in out["matched"] if e["arm"] == arm for q in e["points"]]
    out["g0_"+arm] = {"n_points": len(sp), "median_speed": med(sp), "max_speed": max(sp), "min_speed": min(sp),
                      "steps_ratio_min": min(st), "steps_ratio_max": max(st), "steps_ratio_median": med(st)}
# per-problem ranges
out["ranges"] = []
for p in probs:
    for arm in ("ppsw1_t1", "ppsw1_t6"):
        e = [x for x in out["matched"] if x["problem"] == p and x["arm"] == arm][0]
        ps = [x for x in out["per_step_wall"] if x["problem"] == p]
        key = "ratio_t1_over_rodas5p" if arm == "ppsw1_t1" else "ratio_t6_over_rodas5p"
        out["ranges"].append({"problem": p, "arm": arm,
            "steps_ratio": [min(q["steps_ratio"] for q in e["points"]), max(q["steps_ratio"] for q in e["points"])],
            "speed": [min(q["speed_rodas_over_cand"] for q in e["points"]), max(q["speed_rodas_over_cand"] for q in e["points"])],
            "per_attempt_wall_ratio": [min(x[key] for x in ps), max(x[key] for x in ps)]})
# check t1/t6 states identical
for p in probs:
    for t in tols:
        r1 = [r for r in runs if r["problem"] == p and r["tol"] == t and r["arm"] == "ppsw1_t1"][0]["result"]["y_end"]
        r6 = [r for r in runs if r["problem"] == p and r["tol"] == t and r["arm"] == "ppsw1_t6"][0]["result"]["y_end"]
        for x in out["thread_ratio"]:
            if x["problem"] == p and x["tol"] == t: x["identical_state"] = (r1 == r6)
# stored evaluation comparison
ev = json.loads((H/"EVALUATION.json").read_text())
out["stored_g0"] = {k: {kk: vv for kk, vv in v.items() if kk != "evidence"} for k, v in ev["g0"].items()}
out["stored_speed_evidence_n"] = len(ev["g0"]["median_speed_ge_1_15x"]["evidence"])
out["stored_step_evidence_n"] = len(ev["g0"]["steps_le_1_2x"]["evidence"])
json.dump(out, open("/tmp/claude-1000/dsg-audit/u11_recompute.json", "w"), indent=1, allow_nan=False)
for x in out["per_run"]:
    print(x["problem"], x["tol"], " | ".join(f'{a}: E={x[a]["E"]:.2e} acc={x[a]["accepted"]} rej={x[a]["rejected"]} wall={x[a]["wall"]*1e3:.3f}ms rhs={x[a]["rhs"]} lu={x[a]["lu"]} sol={x[a]["solves"]}' for a in ("rodas5p", "ppsw1_t1", "ppsw1_t6")))
print()
for x in out["per_step_wall"]:
    print(f'{x["problem"]:18s} n={x["n"]:2d} tol={x["tol"]:.0e} us/attempt R={x["rodas5p_us_per_attempt"]:.2f} P1={x["ppsw1_t1_us_per_attempt"]:.2f} P6={x["ppsw1_t6_us_per_attempt"]:.2f} P1/R={x["ratio_t1_over_rodas5p"]:.2f} P6/R={x["ratio_t6_over_rodas5p"]:.2f} sameTol steps={x["same_tol_steps_ratio_t1"]:.2f} wall={x["same_tol_wall_ratio_t1"]:.1f} errRatio={x["same_tol_error_ratio_t1"]:.3g} rhs={x["rhs_ratio_t1"]:.2f} lu={x["lu_ratio_t1"]:.2f} solves={x["solve_ratio_t1"]:.2f}')
print()
for x in out["thread_ratio"]: print(x)
print()
for e in out["matched"]:
    print(e["problem"], e["arm"], "overlap", ["%.3e" % v for v in e["overlap"]], "candE", ["%.2e" % v for v in e["E_range_candidate"]], "rodE", ["%.2e" % v for v in e["E_range_rodas5p"]])
    for q in e["points"]: print("   E*=%.3e steps_ratio=%.3f speed=%.5f" % (q["E_star"], q["steps_ratio"], q["speed_rodas_over_cand"]))
print()
for r in out["ranges"]: print(r)
print(json.dumps({k: out[k] for k in ("g0_ppsw1_t1", "g0_ppsw1_t6", "stored_g0", "stored_speed_evidence_n", "stored_step_evidence_n")}, indent=1))
