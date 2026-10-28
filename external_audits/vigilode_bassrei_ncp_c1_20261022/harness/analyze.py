#!/usr/bin/env python3
"""Analysis and verdict of the BASS/rei NCP C1 campaign (also a library).

    python3 -s analyze.py CAMP

Reads results/{PLAN.json, PARITY.json, LADDER.jsonl, VG.jsonl, ABBA.jsonl,
IDENTITY.json, IDENTITY_END.json, SETUP.json} and the raw stdout files
(resolved relative to CAMP, so a copied-back directory re-scores the same);
writes results/{WORK_PRECISION.json, MATCHED.json, ABBA_SUMMARY.json,
VERDICT.json} and REPORT.md. Works on partial data.

[C] gates:
  G1 references: every main case has the VigilODE 1e-12 reference and the
     Radau 1e-12 cross-check, u_ref <= 1e-7 (B_app metric).
  G2 reference independence: the Python reference RHS matches the VigilODE
     kernel (<= 1e-13 term-scaled) on every case.
  G3 VigilODE at the preregistered rung rtol 1e-8: every VigilODE arm of every
     main case returns the requested outputs with B_app error <= 1e-6; the
     chemistry arms also hold B_src (5e-12 number, 5e-11 eV per H).
  G4 NCP cross-core bit-identity: every preregistered identity group (vg.IDENTITY)
     on 3 distinct cores, validated outputs, equal states/counters digests.
  G5 zero hot-loop allocation: every preregistered allocation group (vg.ALLOCATION:
     equal counts at rtol 1e-6 and 1e-9, calibrated, attempts >= 2x) and
     rejection group (vg.REJECTION: equal counts, more rejected steps with
     initial_step = T).
  G6 coverage: PLAN.json equals the preregistered plan, every item has a row
     with a typed status, and no VigilODE or reference item is NOT_RUN,
     DEADLINE or ENVIRONMENT_FAILURE.
  G7 provenance: the source subset matches the EXEC ls-tree at start and end;
     binaries, Cargo.lock, the Julia project and the diffsol lock unchanged.

Verdict: INVALID when the run is incomplete or its infrastructure failed (G7,
G6, a missing PARITY.json, missing or infrastructure-failed preregistered
VG groups or reference/VigilODE items, setup failure, an unofficial plan);
otherwise PASS iff G1-G7 all pass, else FAIL (a completed [C] measurement
missed its threshold). INVALID writes no ledger row.
[H]: comparator accuracy/cost, unfloored errors, matched rungs (censored at
the loosest rung flagged), ABBA cost ratios (diagnostic_only,
STATISTICAL_AUTHORITY_HOLD).
"""
import json
import math
import os
import statistics
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import scenarios as S  # noqa: E402
from common import read_jsonl, utc_now, write_json  # noqa: E402

TYPED = {"OK", "NUMERICAL_FAILURE", "TIMEOUT", "DEADLINE", "HOLD", "ADAPTER_FAILURE", "OUTPUT_PARSE_FAILURE",
         "REFUSED", "NOT_APPLICABLE", "NOT_RUN", "ENVIRONMENT_FAILURE", "INVALID_PARITY"}
INFRA = {"NOT_RUN", "DEADLINE", "ENVIRONMENT_FAILURE", "OUTPUT_PARSE_FAILURE", None}


class Evaluator:
    def __init__(self, camp):
        self.camp = Path(camp).resolve()
        self.res = self.camp / "results"
        self.ladder = read_jsonl(self.res / "LADDER.jsonl")
        self.parity = json.loads((self.res / "PARITY.json").read_text()) if (self.res / "PARITY.json").exists() else {}
        self.refs = {}
        self.cache = {}

    def path_of(self, row):
        rel = row.get("stdout_rel")
        if rel and (self.camp / rel).exists():
            return self.camp / rel
        p = row.get("stdout_path")
        return Path(p) if p else None

    def load_states(self, row):
        p = self.path_of(row)
        if p is None or not p.exists():
            return None, None, "no output file"
        try:
            doc = json.loads(p.read_text())
        except Exception as e:  # noqa: BLE001
            return None, None, f"unparseable: {e!r}"
        return doc.get("times"), doc.get("states"), None

    def reference(self, cid):
        if cid in self.refs:
            return self.refs[cid]
        d = S.case_data(cid)
        vg = [r for r in self.ladder if r.get("phase") == "ref" and r.get("case") == cid and r.get("arm") != "py_radau_ref"]
        rd = [r for r in self.ladder if r.get("phase") == "ref" and r.get("case") == cid and r.get("arm") == "py_radau_ref"]
        out = {"case": cid, "vigilode": None, "radau": None, "u_ref": None, "ok": False, "infra_missing": False}
        states = {}
        for name, rows in (("vigilode", vg), ("radau", rd)):
            if rows and rows[-1].get("status") == "OK":
                t, s, err = self.load_states(rows[-1])
                bad = err or S.validate_trajectory(t, s, d["outputs"], d["n"])
                out[name] = {"status": "OK" if not bad else "INVALID", "issue": bad, "run_id": rows[-1]["run_id"],
                             "wall_s": (rows[-1].get("process") or {}).get("process_wall_ns", 0) * 1e-9,
                             "timing_authority": "diagnostic_only"}
                if not bad:
                    states[name] = s
            else:
                st = rows[-1].get("status") if rows else None
                out[name] = {"status": st or "MISSING", "reason": rows[-1].get("reason") if rows else None}
                if st in INFRA or st == "TIMEOUT":
                    out["infra_missing"] = True
        if "vigilode" in states and "radau" in states:
            out["u_ref"], out["u_ref_per_observable"] = S.bapp_error(cid, d["outputs"], states["radau"],
                                                                      states["vigilode"])
            out["ok"] = bool(out["u_ref"] <= S.U_REF_MAX)
        out["states"] = states.get("vigilode")
        self.refs[cid] = out
        return out

    def parity_ok(self, cid, arm):
        fam = S.FAMILY_OF_ARM.get(arm)
        adapter = {"vode": "python_reference", "dop853": "python_reference", "julia": "julia",
                   "diffsol": "diffsol"}.get(fam)
        if adapter is None:
            return True, None
        row = ((self.parity.get("cases") or {}).get(cid) or {}).get(adapter) or {}
        return bool(row.get("pass")), row.get("parity")

    def evaluate(self, row):
        key = row.get("run_id")
        if key in self.cache:
            return self.cache[key]
        cid = row["case"]
        d = S.case_data(cid)
        ev = {"run_id": key, "case": cid, "arm": row["arm"], "rtol": row.get("rtol"), "status": row.get("status"),
              "valid": False, "timing_authority": "diagnostic_only"}
        pok, pval = self.parity_ok(cid, row["arm"])
        ev["parity"] = pval
        if not pok:
            ev["status_effective"] = "INVALID_PARITY"
        if row.get("status") == "OK":
            t, s, err = self.load_states(row)
            bad = err or S.validate_trajectory(t, s, d["outputs"], d["n"])
            ref = self.reference(cid)
            if bad:
                ev["refusal"] = bad
            elif ref.get("states") is None:
                ev["refusal"] = "no reference"
            else:
                unf = {}
                e, per = S.bapp_error(cid, d["outputs"], s, ref["states"], unf)
                led = S.ledger_drift(cid, s)
                ev.update(valid=True, error=e, per_observable=per, unfloored_H=unf, ledger=led,
                          b_app_pass=bool(e <= S.B_APP),
                          b_src_pass=led.get("b_src_pass", True) if d["family"] == "chem" else True)
                ev["match"] = bool(ev["b_app_pass"] and ev["b_src_pass"] and pok)
        ev["solve_seconds"] = row.get("solve_seconds")
        ev["timing_scope"] = row.get("timing_scope")
        ev["counters"] = row.get("counters")
        self.cache[key] = ev
        return ev


def ladder_table(E):
    table = {}
    for r in E.ladder:
        if r.get("phase") != "ladder":
            continue
        ev = E.evaluate(r) if r.get("status") == "OK" else {
            "run_id": r.get("run_id"), "case": r.get("case"), "arm": r.get("arm"), "rtol": r.get("rtol"),
            "status": r.get("status"), "reason": r.get("reason"), "valid": False,
            "effective_timeout_s": r.get("effective_timeout_s"), "deadline_capped": r.get("deadline_capped")}
        table.setdefault(r["case"], {}).setdefault(r["arm"], {})[f"{r.get('rtol'):.0e}" if r.get("rtol") else "-"] = ev
    return table


def matched(table):
    """Per (case, arm): the loosest rung meeting B_app (+ B_src) with valid parity;
    a match at the loosest ladder rung is flagged as censored (could be looser)."""
    out = {}
    for cid, arms in table.items():
        for arm, rungs in arms.items():
            ok = [(float(k), v) for k, v in rungs.items() if k != "-" and v.get("match")]
            if ok:
                rt, v = max(ok, key=lambda x: x[0])
                out.setdefault(cid, {})[arm] = {"rtol": rt, "error": v["error"], "solve_seconds": v.get("solve_seconds"),
                                                "timing_scope": v.get("timing_scope"), "counters": v.get("counters"),
                                                "run_id": v["run_id"], "censored_at_loose_edge": rt == max(S.LADDER),
                                                "timing_authority": "diagnostic_only"}
    return out


def vg_groups():
    import vg
    return vg.ALLOCATION, vg.REJECTION, vg.IDENTITY


def gates(E, table):
    res = E.res
    g, infra = {}, []
    plan_doc = json.loads((res / "PLAN.json").read_text()) if (res / "PLAN.json").exists() else {}
    planned = plan_doc.get("cases") or list(S.CASE)
    main = [c for c in planned if not S.CASE[c]["auxiliary"]]
    if not plan_doc:
        infra.append("no PLAN.json")
    if plan_doc and not plan_doc.get("official"):
        infra.append(f"unofficial plan (overrides {plan_doc.get('overrides')})")
    # G1
    refs = {c: E.reference(c) for c in main}
    for c, r in refs.items():
        if r.get("infra_missing") or r["vigilode"] is None or r["radau"] is None:
            infra.append(f"reference {c} missing or cut ({(r['vigilode'] or {}).get('status')}/"
                         f"{(r['radau'] or {}).get('status')})")
    g["G1_references"] = {"pass": all(r["ok"] for r in refs.values()) and bool(refs),
                          "values": {c: {k: v for k, v in r.items() if k != "states"} for c, r in refs.items()}}
    # G2
    par = E.parity.get("cases") or {}
    if not E.parity or E.parity.get("partial", True):
        infra.append("PARITY.json missing or partial")
    pv = {c: (par.get(c) or {}).get("python_reference") for c in planned}
    g["G2_reference_parity"] = {"pass": all(bool(v and v.get("pass")) for v in pv.values()), "values": pv}
    # G3
    g3, ok3 = {}, True
    for c in main:
        for a in S.arms_for(c):
            if a["arm"] not in S.VIGILODE_ARMS or not a["rungs"]:
                continue
            ev = (table.get(c, {}).get(a["arm"], {}) or {}).get(f"{S.RUNG:.0e}")
            if ev is None or ev.get("status") in INFRA:
                infra.append(f"VigilODE {c}/{a['arm']} rung item {(ev or {}).get('status', 'missing')}")
            good = bool(ev and ev.get("valid") and ev.get("b_app_pass") and ev.get("b_src_pass"))
            g3[f"{c}/{a['arm']}"] = {"error": (ev or {}).get("error"), "status": (ev or {}).get("status"),
                                     "margin_B_app_over_error": (S.B_APP / ev["error"]) if ev and ev.get("error") else None,
                                     "ledger": (ev or {}).get("ledger"), "pass": good}
            ok3 &= good
    g["G3_vigilode_rung"] = {"pass": bool(ok3 and g3), "values": g3}
    # G4, G5 against the preregistered group lists
    vgrows = read_jsonl(res / "VG.jsonl")
    ALLOC, REJ, IDENT = vg_groups()

    def rows_of(kind, grp):
        return [r for r in vgrows if r.get("kind") == kind and r.get("group") == grp]
    ident, ok4 = {}, True
    for cid, arm in IDENT:
        grp = f"{cid}/{arm}"
        rows = rows_of("identity", grp)
        if len(rows) < 3 or any(r.get("status") in INFRA for r in rows):
            infra.append(f"identity group {grp} incomplete")
        digests = {(r.get("states_sha256"), r.get("counters_sha256")) for r in rows}
        cores = {r.get("core") for r in rows}
        good = (len(rows) == 3 and all(r.get("status") == "OK" and r.get("trajectory_refusal") is None for r in rows)
                and len(cores) == 3 and len(digests) == 1)
        ident[grp] = {"cores": sorted(c for c in cores if c is not None), "distinct_digests": len(digests),
                      "pass": bool(good)}
        ok4 &= good
    g["G4_cross_core_identity"] = {"pass": bool(ok4), "values": ident}
    alloc, ok5 = {}, True
    for kind, groups in (("allocation", ALLOC), ("rejection", REJ)):
        for cid, arm in groups:
            grp = f"{cid}/{arm}"
            rows = rows_of(kind, grp)
            if len(rows) < 2 or any(r.get("status") in INFRA for r in rows):
                infra.append(f"{kind} group {grp} incomplete")
            al = [(r.get("allocations") or {}) for r in rows]
            det = [(r.get("details") or {}) for r in rows]
            att = [x.get("attempts") or 0 for x in det]
            rej = [x.get("rejected_steps") or 0 for x in det]
            good = (len(rows) == 2 and all(r.get("status") == "OK" for r in rows) and all(a.get("calibrated") for a in al)
                    and al[0].get("integrate") == al[1].get("integrate"))
            if kind == "allocation":
                good = good and max(att) >= 2 * max(min(att), 1)
            else:
                good = good and rej[1] > rej[0]
            alloc[f"{kind}:{grp}"] = {"integrate": [a.get("integrate") for a in al], "attempts": att,
                                      "rejected_steps": rej, "pass": bool(good)}
            ok5 &= good
    g["G5_allocation"] = {"pass": bool(ok5), "values": alloc}
    # G6
    seen = {}
    for r in E.ladder:
        seen[(r.get("case"), r.get("arm"), r.get("rtol"), r.get("phase"))] = r.get("status")
    missing, untyped, vg_bad = [], [], []
    for it in plan_doc.get("items", []):
        st = seen.get((it["case"], it["arm"], it["rtol"], it["phase"]))
        if st is None:
            missing.append(it["run_id"])
        elif st not in TYPED:
            untyped.append((it["run_id"], st))
        if (it["arm"] in S.VIGILODE_ARMS or it["phase"] == "ref") and (st in INFRA) \
                and not S.CASE[it["case"]]["auxiliary"]:
            vg_bad.append((it["run_id"], st))
    counts = {}
    for st in seen.values():
        counts[st] = counts.get(st, 0) + 1
    g6 = bool(plan_doc.get("items")) and not missing and not untyped and not vg_bad and bool(plan_doc.get("official"))
    if missing or untyped or vg_bad:
        infra.append(f"coverage: {len(missing)} missing, {len(untyped)} untyped, {len(vg_bad)} VigilODE/reference cut")
    g["G6_coverage"] = {"pass": g6, "values": {"planned": len(plan_doc.get("items", [])), "rows": len(seen),
                                              "status_counts": counts, "missing": missing[:50],
                                              "untyped": untyped[:50], "vigilode_or_reference_cut": vg_bad[:50],
                                              "official_plan": plan_doc.get("official")}}
    # G7
    st0 = json.loads((res / "IDENTITY.json").read_text()) if (res / "IDENTITY.json").exists() else {}
    st1 = json.loads((res / "IDENTITY_END.json").read_text()) if (res / "IDENTITY_END.json").exists() else {}
    keys = ("binaries", "cargo_lock_sha256", "julia_project_sha256", "diffsol_lock_sha256")
    prov = bool(st0.get("tree_matches_expected") and st1.get("tree_matches_expected")
                and all(st0.get(k) == st1.get(k) for k in keys))
    if not prov:
        infra.append("provenance (G7) failed or IDENTITY_END missing")
    g["G7_provenance"] = {"pass": prov, "values": {"start": st0, "end": st1, "compared": keys}}
    setup = json.loads((res / "SETUP.json").read_text()) if (res / "SETUP.json").exists() else {}
    if setup.get("runner_build_exit", 1) != 0:
        infra.append("setup/runner build failed or SETUP.json missing")
    return g, infra


def abba_summary(camp_results):
    rows = read_jsonl(Path(camp_results) / "ABBA.jsonl")
    plan = json.loads((Path(camp_results) / "ABBA_PLAN.json").read_text()) \
        if (Path(camp_results) / "ABBA_PLAN.json").exists() else {}
    meta = {p["pair"]: p for p in plan.get("pairs", [])}
    out = {}
    for pair in sorted({r.get("pair") for r in rows} - {None}):
        pr = [r for r in rows if r.get("pair") == pair and not r.get("warmup")]
        status_counts = {}
        for r in pr:
            status_counts[r.get("status")] = status_counts.get(r.get("status"), 0) + 1
        blocks = {}
        for r in pr:
            if r.get("status") == "OK":
                blocks.setdefault(r["block_index"], {})[r["position_arm"]] = r.get("solve_seconds")
        ratios = [b["B"] / b["A"] for b in blocks.values() if b.get("A") and b.get("B")]
        m = meta.get(pair, {})
        out[pair] = {"confirmation_blocks_ok": len(ratios), "confirmation_rows_by_status": status_counts,
                     "complete": len(ratios) >= 7,
                     "median_B_over_A": statistics.median(ratios) if ratios else None,
                     "min": min(ratios) if ratios else None, "max": max(ratios) if ratios else None,
                     "A": m.get("A"), "B": m.get("B"), "error_ratio_B_over_A": m.get("error_ratio_B_over_A"),
                     "skipped_reason": None,
                     "timing_authority": "diagnostic_only", "statistical_authority": "STATISTICAL_AUTHORITY_HOLD"}
    for p in plan.get("skipped", []):
        out[p["pair"]] = {"skipped_reason": f"matched run {p.get('matched_run_seconds', 0):.0f} s > "
                                            f"{plan.get('max_run_seconds')} s", "A": p.get("A"), "B": p.get("B"),
                          "timing_authority": "diagnostic_only"}
    return out


def main():
    camp = Path(sys.argv[1]).resolve()
    res = camp / "results"
    os.environ["NCPC1_RESULTS"] = str(res)
    E = Evaluator(camp)
    table = ladder_table(E)
    write_json(res / "WORK_PRECISION.json", {"generated_utc": utc_now(), "B_app": S.B_APP, "table": table,
                                              "timing_authority": "diagnostic_only"})
    m = matched(table)
    write_json(res / "MATCHED.json", {"matched": m, "timing_authority": "diagnostic_only"})
    g, infra = gates(E, table)
    ab = abba_summary(res)
    write_json(res / "ABBA_SUMMARY.json", ab)
    if infra:
        verdict = "INVALID"
    else:
        verdict = "PASS" if all(v["pass"] for v in g.values()) else "FAIL"
    write_json(res / "VERDICT.json", {"generated_utc": utc_now(), "verdict": verdict,
                                      "infrastructure_issues": infra,
                                      "gate_table": {k: v["pass"] for k, v in g.items()}, "gates": g,
                                      "timing_authority": "diagnostic_only"})
    lines = [f"# BASS/rei NCP C1 campaign: {verdict}", ""]
    if infra:
        lines += ["Infrastructure issues (verdict INVALID, no ledger row):", ""] + [f"- {x}" for x in infra[:40]] + [""]
    lines += ["| gate | pass |", "|---|---|"] + [f"| {k} | {v['pass']} |" for k, v in g.items()]
    lines += ["", "## Matched rungs (loosest rtol meeting B_app/B_src; solve seconds diagnostic_only; * = censored at "
              "the loosest rung)", ""]
    for cid, arms in sorted(m.items()):
        best = sorted(arms.items(), key=lambda kv: (kv[1]["solve_seconds"] if kv[1]["solve_seconds"] is not None
                                                    else math.inf))
        lines.append(f"- {cid}: " + ", ".join(
            f"{a} rtol {v['rtol']:.0e}{'*' if v['censored_at_loose_edge'] else ''} {v['solve_seconds']:.3g}s"
            for a, v in best[:8] if v["solve_seconds"] is not None))
    lines += ["", "## ABBA (B/A solve-time medians, diagnostic_only)", ""]
    for p, v in ab.items():
        if v.get("skipped_reason"):
            lines.append(f"- {p}: skipped ({v['skipped_reason']})")
        else:
            lines.append(f"- {p}: {v['median_B_over_A']} over {v['confirmation_blocks_ok']} OK blocks "
                         f"{v['confirmation_rows_by_status']}{'' if v['complete'] else ' (incomplete)'}")
    (camp / "REPORT.md").write_text("\n".join(lines) + "\n")
    print(json.dumps({"verdict": verdict, "gates": {k: v["pass"] for k, v in g.items()}, "infra": infra[:10]}))


if __name__ == "__main__":
    main()
