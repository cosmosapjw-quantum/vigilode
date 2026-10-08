"""B6 analysis: per-row table, confusion matrices, kill criteria, attribution controls, cost models.
EXPLORATORY.  Reads rows/*.json written by geamp.py; writes summary.json and prints the tables."""
import os, sys, json, math, glob
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
THRESH, GUARD = 4.0, 0.1
IN_SAMPLE = {"corpus:semilinear:1e-4", "corpus:semilinear:1e-6", "corpus:semilinear:1e-8"} | \
    {f"bench:{p}:{r}" for p in ("robertson", "hires", "vdp", "bruss50") for r in ("1e-4", "1e-6", "1e-8")}
ORDER_C = ["robertson", "hires", "vdp", "rotating", "forcing", "semilinear"]
ORDER_B = ["robertson", "hires", "vdp", "bruss50", "bruss200"]


def load():
    rows = {}
    for fn in glob.glob(os.path.join(HERE, "rows", "*.json")):
        d = json.load(open(fn)); rows[d["key"]] = d
    return rows


def sortkey(k):
    kind, name, r = k.split(":")
    o = ORDER_C if kind == "corpus" else ORDER_B
    return (0 if kind == "corpus" else 1, o.index(name), -float(r))


def flops_models(d):
    n = d["n"]; kl, ku, nnz = d["bandwidth"]["kl"], d["bandwidth"]["ku"], d["bandwidth"]["nnz"]
    rf = d["rhs_flops"]
    lu_d, bs_d, mv_d, wa_d = 2 / 3 * n ** 3, 2.0 * n ** 2, 2.0 * n ** 2, float(n ** 2)
    lu_b = 2.0 * n * kl * ku + n * kl
    bs_b = 2.0 * n * kl + 2.0 * n * (kl + ku) + n
    mv_b, wa_b = 2.0 * nnz, float(nnz)

    def base(cn, lu, bs, wa):
        return (cn["lu"] * (lu + wa) + cn["backsolve"] * bs + (cn["rhs"] + cn["ft"]) * rf + cn["jac"] * 2.0 * nnz
                + cn["stage_axpy"] * 2.0 * n + cn["wrms"] * 3.0 * n)

    cn = d["counters"]; ov = d["overhead"]
    out = {}
    for lab, lu, bs, mv, wa in (("dense", lu_d, bs_d, mv_d, wa_d), ("banded", lu_b, bs_b, mv_b, wa_b)):
        b = base(cn, lu, bs, wa)
        o5 = ov["res5"]["backsolve"] * bs + ov["res5"]["axpy"] * 2.0 * n + ov["res5"]["wrms"] * 3.0 * n
        oR = ov["exact"]["backsolve"] * bs + ov["exact"]["matvec"] * mv + ov["exact"]["axpy"] * 2.0 * n + \
            ov["exact"]["wrms"] * 3.0 * n
        rv = base(d["rival"]["counters"], lu, bs, wa)
        out[lab] = {"base_flops": b, "res5_overhead": o5 / b, "exact_overhead": oR / b, "rival_overhead": rv / b,
                    "res5_backsolve_ratio": ov["res5"]["backsolve"] / cn["backsolve"]}
    # matrix-free: Rust recorded baseline (corpus rows), replica-measured transport GMRES scaled per accepted step
    if "rust" in d:
        ru = d["rust"]; jf = d["rhs_flops"]
        base_mf = (ru["jvp_vectors"] * jf + (ru["rhs_evaluations"] + ru["ft_calls"]) * rf
                   + (ru["orthogonalization_inner_products"] + ru["orthogonalization_vector_updates"]) * 2.0 * n
                   + ru["attempts"] * (72 * 2.0 * n + 3.0 * n))
        acc_rep = cn["accepted"]; scale = ru["accepted"] / acc_rep
        mfo = {}
        for lab in ("res5_mf", "exact_mf", "res5_mf4", "exact_mf4"):
            o = ov[lab]
            fl = (o.get("jvp", 0) * jf + (o.get("dot", 0) + o.get("axpy", 0)) * 2.0 * n + acc_rep * (6 * 2.0 * n + 3.0 * n))
            mfo[lab] = {"flop_overhead": fl * scale / base_mf, "jvp_overhead": o.get("jvp", 0) * scale / ru["jvp_vectors"],
                        "jvp_per_accepted": o.get("jvp", 0) / acc_rep}
        rv_inf = d["rival"]["attempts"] / cn["attempts"]
        out["matrix_free"] = {"base_flops_rust": base_mf, **mfo, "rival_overhead_inferred_attempt_ratio": rv_inf,
                              "rust_jvp_per_attempt": ru["jvp_vectors"] / ru["attempts"]}
    return out


def attribution(d):
    st = d["steps"]
    le = np.array(st["le"]); err = np.array(st["err"]); G = np.array(st["G"])
    RRt = np.array(st["RRt"]); cle = np.cumsum(le)
    with np.errstate(all="ignore"):
        A_truele = np.where(cle > 0, RRt / np.where(cle > 0, cle, 1.0), np.nan)                      # transport of TRUE local errors / sum true local (ref weights)
    eff = err / np.maximum(le, 1e-300)        # estimate / true local (mixed weights: step vs ref)
    ok = le > 1e-6 * max(le.max(), 1e-300)
    return {"A_true_driven_exact_max": float(np.nanmax(A_truele)),
            "R_run_max": float(np.nanmax(st["Rrun"])),
            "transport_true_over_G": float(RRt.max() / G.max()),
            "transport_est_over_G": float(np.max(st["gR_ref"]) / G.max()),
            "effectivity_p05_p95": [float(np.percentile(eff[ok], 5)), float(np.percentile(eff[ok], 95))] if ok.any() else None,
            "effectivity_spread": float(np.percentile(eff[ok], 95) / np.percentile(eff[ok], 5)) if ok.any() else None}


def main():
    rows = load()
    keys = sorted(rows, key=sortkey)
    table = []
    for k in keys:
        d = rows[k]; tr = d["truth"]; de = d["detector"]
        exceed = d["grid"]["max_grid_case"] > 1.0 if d["kind"] == "corpus" else tr["G_max_nodes"] > 1.0
        amp = tr["R_tot"] > 4.0
        r = {"key": k, "in_sample": k in IN_SAMPLE, "att": d["counters"]["attempts"], "acc": d["counters"]["accepted"],
             "grid_max": d["grid"]["max_grid_case"], "G_nodes": tr["G_max_nodes"], "sum_le": tr["sum_le"],
             "max_le": tr["max_le"], "R_tot": tr["R_tot"], "R_run": tr["R_run_max"],
             "A5": de["res5"]["A_max"], "AR": de["exact"]["A_max"], "A5mf": de["res5_mf"]["A_max"],
             "ARmf": de["exact_mf"]["A_max"], "A5mf4": de["res5_mf4"]["A_max"], "ARmf4": de["exact_mf4"]["A_max"],
             "guard5": de["res5"]["guard_value"], "guardR": de["exact"]["guard_value"],
             "F5": de["res5"]["FLAG"], "FR": de["exact"]["FLAG"], "INV5": de["res5"]["INVALID"], "INVR": de["exact"]["INVALID"],
             "AMP5": de["res5"]["AMP"], "AMPR": de["exact"]["AMP"],
             "F5mf": de["res5_mf"]["A_max"] > THRESH or de["res5"]["INVALID"],
             "FRmf": de["exact_mf"]["A_max"] > THRESH or de["exact"]["INVALID"],
             "transport_max_tol": de["exact"]["max_transport_tol_units"], "transport5_max_tol": de["res5"]["max_transport_tol_units"],
             "exceeds": bool(exceed), "amplifying": bool(amp),
             "rival_est": d["rival"]["ge_est_max_grid"], "rival_flag": d["rival"]["FLAG_exceed"],
             "rival_est_over_true": d["rival"]["est_over_true"],
             "ref_check_case": d.get("ref_check_case"), "rust_grid": d.get("rust", {}).get("max_grid_case"),
             "rust_att": d.get("rust", {}).get("attempts"), "rust_rej": d.get("rust", {}).get("rejected"),
             "rej": d["counters"]["rejected"], "sum_le_exact_state": tr.get("sum_le_exact_state"),
             "cost": flops_models(d), "attr": attribution(d), "wall": d.get("wall_total")}
        table.append(r)
    # ---- confusion matrices
    conf = {}

    def cm(rows_, flag, truth):
        tp = sum(1 for r in rows_ if r[flag] and r[truth]); fp = sum(1 for r in rows_ if r[flag] and not r[truth])
        fn = sum(1 for r in rows_ if (not r[flag]) and r[truth]); tn = sum(1 for r in rows_ if (not r[flag]) and not r[truth])
        return {"TP": tp, "FP": fp, "FN": fn, "TN": tn,
                "FP_rows": [r["key"] for r in rows_ if r[flag] and not r[truth]],
                "FN_rows": [r["key"] for r in rows_ if (not r[flag]) and r[truth]]}
    subsets = {"all": table, "out_of_sample": [r for r in table if not r["in_sample"]],
               "corpus_18": [r for r in table if r["key"].startswith("corpus") and not r["key"].endswith("1e-10")],
               "corpus_all": [r for r in table if r["key"].startswith("corpus")]}
    for sname, sub in subsets.items():
        for flag in ("F5", "FR", "AMP5", "AMPR", "F5mf", "FRmf", "rival_flag"):
            for truth in ("amplifying", "exceeds"):
                conf[f"{sname}|{flag}|{truth}"] = cm(sub, flag, truth)
    # ---- kill criteria
    kills = {"K1_false_negative": [], "K2_false_positive": [], "K3_res5_vs_exact": []}
    for r in table:
        for lab, a, inv in (("res5", r["A5"], r["INV5"]), ("exact", r["AR"], r["INVR"])):
            if r["R_tot"] > 5 and a < 3:
                kills["K1_false_negative"].append((r["key"], lab, r["R_tot"], a))
            if r["R_tot"] < 2 and a > 4 and not inv:
                kills["K2_false_positive"].append((r["key"], lab, r["R_tot"], a))
        if r["F5"] != r["FR"]:
            kills["K3_res5_vs_exact"].append((r["key"], r["A5"], r["AR"], r["guard5"], r["guardR"]))
    summary = {"rows": table, "confusion": conf, "kills": kills}
    json.dump(summary, open(os.path.join(HERE, "summary.json"), "w"), indent=1)
    # ---- print
    print(f"{'row':26s} {'IS':2s} {'att':>5s} {'acc':>5s} {'grid':>8s} {'Gnode':>8s} {'sumle':>7s} {'R_tot':>7s} {'R_run':>7s} "
          f"{'A5':>8s} {'AR':>8s} {'A5mf':>7s} {'ARmf':>7s} {'guard5':>8s} {'F5':>3s} {'FR':>3s} {'exc':>3s} {'amp':>3s} {'rival':>7s} {'refchk':>8s}")
    for r in table:
        print(f"{r['key']:26s} {'*' if r['in_sample'] else ' ':2s} {r['att']:5d} {r['acc']:5d} {r['grid_max']:8.3g} {r['G_nodes']:8.3g} "
              f"{r['sum_le']:7.3g} {r['R_tot']:7.3g} {r['R_run']:7.3g} {r['A5']:8.3g} {r['AR']:8.3g} {r['A5mf']:7.3g} {r['ARmf']:7.3g} "
              f"{r['guard5']:8.2g} {int(r['F5']):3d} {int(r['FR']):3d} {int(r['exceeds']):3d} {int(r['amplifying']):3d} "
              f"{r['rival_est']:7.3g} {r['ref_check_case'] if r['ref_check_case'] is None else format(r['ref_check_case'], '8.2g')}")
    print("\nconfusion (TP/FP/FN/TN):")
    for k, v in conf.items():
        print(f"  {k:40s} {v['TP']:3d} {v['FP']:3d} {v['FN']:3d} {v['TN']:3d}  FP={v['FP_rows']} FN={v['FN_rows']}")
    print("\nkills:", json.dumps(kills, indent=1))
    print("\ncost (overhead fraction of baseline flops):")
    print(f"{'row':26s} {'d_res5':>7s} {'d_exact':>7s} {'d_rival':>7s} {'b_res5':>7s} {'b_exact':>7s} {'b_rival':>7s} "
          f"{'mf_r5':>7s} {'mf_ex':>7s} {'mf_r5_4':>7s} {'mf_ex_4':>7s} {'mfJVP5':>7s} {'mf_riv':>7s}")
    for r in table:
        c = r["cost"]; m = c.get("matrix_free")
        s = (f"{r['key']:26s} {c['dense']['res5_overhead']:7.3f} {c['dense']['exact_overhead']:7.3f} {c['dense']['rival_overhead']:7.3f} "
             f"{c['banded']['res5_overhead']:7.3f} {c['banded']['exact_overhead']:7.3f} {c['banded']['rival_overhead']:7.3f}")
        if m:
            s += (f" {m['res5_mf']['flop_overhead']:7.3f} {m['exact_mf']['flop_overhead']:7.3f} {m['res5_mf4']['flop_overhead']:7.3f} "
                  f"{m['exact_mf4']['flop_overhead']:7.3f} {m['res5_mf']['jvp_overhead']:7.3f} {m['rival_overhead_inferred_attempt_ratio']:7.3f}")
        print(s)
    print("\nattribution:")
    for r in table:
        a = r["attr"]
        print(f"{r['key']:26s} A5 {r['A5']:8.3g} AR {r['AR']:8.3g} A_true-driven {a['A_true_driven_exact_max']:8.3g} R_run {a['R_run_max']:8.3g} "
              f"T_true/G {a['transport_true_over_G']:8.3g} T_est/G {a['transport_est_over_G']:8.3g} eff p5-p95 {a['effectivity_p05_p95']} spread {a['effectivity_spread']}")


if __name__ == "__main__":
    main()
