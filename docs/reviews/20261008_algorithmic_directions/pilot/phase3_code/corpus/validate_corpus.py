"""Probe A2 validation of corpus_v2.py (EXPLORATORY, not ledger authority).
(a) mpmath oracles + analytic JVP vs FD (+ dense J, sparse J, f_t, manufactured residual)
(b) SciPy Radau ladder at n = 96 vs the stored selected_raw references; Rust selected_raw rtol-1e-8 trajectories
    re-scored with this module's error rule vs the recorded Rust metrics; E-03 scipy arm A reproduced on 18 rows
(c) per-family operator table (rho, alpha, mu_2, Henrici, sector angles) at y0, mid-span and over the grid,
    and the 18-row map to the recorded Rust v2 campaign (F-033).
Writes results.json next to this file."""
import os
for _v in ("OPENBLAS_NUM_THREADS", "OMP_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "1"
import json, math, sys, time
import numpy as np
import corpus_v2 as cv

HERE = os.path.dirname(os.path.abspath(__file__))
E03 = "/home/user/wt-speed/research/adversarial_audit_20260927/experiments/E-03/scipy"
OUT = {}


def section_a():
    rng = np.random.default_rng(20261008)
    res = {"oracle": cv._check_oracles(verbose=False), "fd": {}}
    for n in (96, 384, 1536):
        for fam in cv.CALIBRATION_FAMILIES:
            p = cv.build(fam, n)
            res["fd"][p.name] = cv._fd_checks(p, rng, verbose=False)
    return res


def section_b_refs():
    out = {}
    for fam in cv.CALIBRATION_FAMILIES:
        p = cv.build(fam, 96)
        st = p.reference()
        stored_ev = json.load(open(st["source"]))["run_evidence"]
        lv = {}
        for lab, rt, at in (("L0", 1e-8, 1e-10), ("L1", 1e-10, 1e-12), ("L2", 1e-12, 1e-14)):
            t0 = time.perf_counter()
            ts, Y, info = cv.radau_reference(p, rt, at)
            lv[lab] = (Y, info, time.perf_counter() - t0)
        Y2 = lv["L2"][0]
        Wst = lambda A, B: float(cv.wrms_rows(A, B).max())
        anchor = lambda A, B: float((np.sqrt(np.mean(((A - B) / (1e-10 + 1e-8 * np.abs(Y2))) ** 2, axis=1))).max())
        d0, d1 = anchor(lv["L0"][0], lv["L1"][0]), anchor(lv["L1"][0], Y2)
        q = d1 / d0
        r = {
            "stored_uncertainty_wrms": st["uncertainty_wrms"], "stored_d1": st["d1"], "stored_q": st["q"],
            "mine_d0": d0, "mine_d1": d1, "mine_q": q, "mine_richardson": d1 * q / (1 - q),
            "L2_vs_stored_max_grid_wrms": Wst(Y2, st["states"]),
            "L2_vs_stored_endpoint_wrms": float(cv.wrms_rows(Y2[-1], st["states"][-1])[0]),
            "L2_bitwise_equal_points": int(sum(np.array_equal(Y2[i], st["states"][i]) for i in range(len(Y2)))),
            "L2_nfev_nlu_mine": [lv["L2"][1]["nfev"], lv["L2"][1]["nlu"]],
            "L2_nfev_nlu_stored": [e for e in stored_ev if e["label"] == "L2"][0] and
                                  [[e["nfev"], e["nlu"]] for e in stored_ev if e["label"] == "L2"][0],
            "L2_wall_s": lv["L2"][2],
        }
        r["agreement_over_uncertainty"] = r["L2_vs_stored_max_grid_wrms"] / st["uncertainty_wrms"]
        if p.exact is not None:
            E = np.array([p.exact(t) for t in p.output_times])
            r["stored_vs_exact_max_grid_wrms"] = Wst(st["states"], E)
            r["mine_vs_exact_max_grid_wrms"] = Wst(Y2, E)
        out[p.name] = r
        print(f"[b-ref] {p.name}: vs stored {r['L2_vs_stored_max_grid_wrms']:.3e} (unc {st['uncertainty_wrms']:.3e}), "
              f"d1 {d1:.3e} vs stored {st['d1']:.3e}, nfev/nlu {r['L2_nfev_nlu_mine']} vs {r['L2_nfev_nlu_stored']}",
              flush=True)
    return out


def section_b_rust_raw():
    """Re-score the Rust selected_raw rtol 1e-8 trajectories with this module's rule."""
    raw = json.load(open(cv.STORED_RUST_RAW_N96_1E8))
    out = {}
    for rec in raw["records"]:
        a = rec["artifact"]; spec = a["spec"]
        fam = spec["family"]
        p = cv.build(fam, spec["dimension"])
        st = p.reference()
        _, Ymine, _ = cv.radau_reference(p, 1e-12, 1e-14)
        row = {}
        for arm in ("clipped", "dense"):
            Y = np.array(a[arm]["states"]); T = np.array(a[arm]["output_times"])
            assert np.allclose(T, p.output_times, rtol=0, atol=1e-15)
            m = cv.global_error_metrics(Y, st["states"], spec["rtol"], st["uncertainty_wrms"])
            rec_m = a[arm]["metrics"]
            m2 = cv.global_error_metrics(Y, Ymine, spec["rtol"])
            row[arm] = {
                "recorded_max_grid_wrms": rec_m["max_grid_wrms"], "rescored_vs_stored": m["max_grid_wrms"],
                "rel_diff_recorded": abs(m["max_grid_wrms"] - rec_m["max_grid_wrms"]) / rec_m["max_grid_wrms"],
                "recorded_endpoint_wrms": rec_m["endpoint_wrms"], "rescored_endpoint": m["endpoint_wrms"],
                "recorded_rms_grid_wrms": rec_m["rms_grid_wrms"], "rescored_rms_grid": m["rms_grid_wrms"],
                "vs_my_radau_max_grid_wrms": m2["max_grid_wrms"],
                "case_units": m["max_grid_case"],
            }
        out[spec["id"]] = row
        print(f"[b-rust] {spec['id']}: dense recorded {row['dense']['recorded_max_grid_wrms']:.6e} rescored "
              f"{row['dense']['rescored_vs_stored']:.6e} vs-mine {row['dense']['vs_my_radau_max_grid_wrms']:.6e}", flush=True)
    return out


def section_b_e03():
    """E-03 arm A: Radau, case (rtol, atol), first_step = span/100, dense output on the grid, analytic J."""
    out = {}
    for fam in cv.CALIBRATION_FAMILIES:
        p = cv.build(fam, 96)
        st = p.reference()
        e03 = json.load(open(os.path.join(E03, fam + ".json")))
        for rr in e03["rtol_rows"]:
            rtol, atol = rr["rtol"], rr["atol"]
            A = rr["arms"]["A_h0_dense"]
            h0 = (p.span[1] - p.span[0]) / 100.0
            ts, Y, info = cv.radau_reference(p, rtol, atol, first_step=h0, dense_output=True)
            m = cv.global_error_metrics(Y, st["states"], rtol, st["uncertainty_wrms"])
            out[rr["case_id"]] = {
                "e03_max_grid_case": A["max_grid_wrms_casetol"], "mine_max_grid_case": m["max_grid_case"],
                "e03_nfev": A["nfev"], "mine_nfev": info["nfev"], "e03_steps": A["steps"], "mine_steps": info["steps"],
                "e03_nlu": A["nlu"], "mine_nlu": info["nlu"],
                "e03_endpoint_case": A["endpoint_wrms_casetol"], "mine_endpoint_case": m["endpoint_case"],
            }
            o = out[rr["case_id"]]
            print(f"[b-e03] {rr['case_id']}: case err e03 {o['e03_max_grid_case']:.4g} mine {o['mine_max_grid_case']:.4g}; "
                  f"steps {o['e03_steps']}/{o['mine_steps']} nfev {o['e03_nfev']}/{o['mine_nfev']}", flush=True)
    return out


def section_c_ops(n=96, ref_states=None):
    out = {}
    for fam in cv.CALIBRATION_FAMILIES:
        p = cv.build(fam, n)
        r = p.reference()
        T = p.output_times
        if r is None:
            out[p.name] = {"note": "no reference"}; continue
        S = r["states"]
        stats = []
        for k, t in enumerate(T):
            Jm = p.J(t, S[k])
            stats.append(cv.operator_stats(Jm))
        mid = len(T) // 2
        rho = np.array([s["rho"] for s in stats]); mu2 = np.array([s["mu2"] for s in stats])
        alpha = np.array([s["alpha"] for s in stats]); hen = np.array([s["henrici_rel"] for s in stats])
        sect = np.array([s["eig_sector_deg"] for s in stats])
        kmu = int(np.argmax(mu2)); krho = int(np.argmax(rho))
        fov = {}
        for lab, k in (("y0", 0), ("mid", mid), ("argmax_mu2", kmu)):
            fov[lab] = cv.operator_stats(p.J(T[k], S[k]), fov_angles=180)
            fov[lab]["t"] = float(T[k])
        ev0 = np.linalg.eigvals(p.J(T[0], S[0])); evm = np.linalg.eigvals(p.J(T[mid], S[mid]))

        def sratio(ev):
            re = -ev.real[ev.real < 0]
            return float(re.max() / re.min()) if len(re) else float("nan")
        out[p.name] = {
            "n": p.n, "span": p.span, "autonomous": p.autonomous, "exact_solution": p.exact is not None,
            "grid_shape": p.grid_shape,
            "y0": fov["y0"], "mid": fov["mid"], "at_max_mu2": fov["argmax_mu2"],
            "max_rho": float(rho.max()), "t_max_rho": float(T[krho]), "max_mu2": float(mu2.max()), "t_max_mu2": float(T[kmu]),
            "min_mu2": float(mu2.min()), "max_alpha": float(alpha.max()), "t_max_alpha": float(T[int(np.argmax(alpha))]),
            "frac_grid_mu2_pos": float(np.mean(mu2 > 0)), "frac_grid_alpha_pos": float(np.mean(alpha > 0)),
            "max_henrici_rel": float(hen.max()), "max_eig_sector_deg": float(sect.max()),
            "rho_times_span_max": float(rho.max() * (p.span[1] - p.span[0])),
            "stiffness_ratio_y0": sratio(ev0), "stiffness_ratio_mid": sratio(evm),
            "grid_rho": rho.tolist(), "grid_mu2": mu2.tolist(), "grid_alpha": alpha.tolist(),
        }
        o = out[p.name]
        print(f"[c] {p.name}: rho0={o['y0']['rho']:.3g} rho_mid={o['mid']['rho']:.3g} maxrho={o['max_rho']:.3g}@{o['t_max_rho']:.3g} "
              f"mu2_0={o['y0']['mu2']:.3g} mu2_mid={o['mid']['mu2']:.3g} maxmu2={o['max_mu2']:.3g}@{o['t_max_mu2']:.3g} "
              f"maxalpha={o['max_alpha']:.3g} henrel0={o['y0']['henrici_rel']:.3g} henrel_mid={o['mid']['henrici_rel']:.3g}",
              flush=True)
    return out


def section_c_ops_large(n):
    """y0 and mid-span operator statistics at n = 384 / 1536 (references generated by gen_refs.py)."""
    out = {}
    for fam in cv.CALIBRATION_FAMILIES:
        p = cv.build(fam, n)
        r = p.reference()
        T = p.output_times; mid = len(T) // 2
        o = {"n": n, "grid_shape": p.grid_shape, "reference": None if r is None else r["source"],
             "reference_uncertainty_wrms": None if r is None else r["uncertainty_wrms"]}
        o["y0"] = cv.operator_stats(p.J(T[0], p.y0))
        if r is not None:
            o["mid"] = cv.operator_stats(p.J(T[mid], r["states"][mid]))
            o["end"] = cv.operator_stats(p.J(T[-1], r["states"][-1]))
        out[p.name] = o
        print(f"[c-{n}] {p.name}: rho0={o['y0']['rho']:.4g} mu2_0={o['y0']['mu2']:.4g} "
              + (f"rho_mid={o['mid']['rho']:.4g} mu2_mid={o['mid']['mu2']:.4g} alpha_mid={o['mid']['alpha']:.4g} "
                 f"henrel_mid={o['mid']['henrici_rel']:.3f} rho_end={o['end']['rho']:.4g}" if r is not None else ""),
              flush=True)
    return out


def section_d_amplification(n=96):
    """Linearized error propagator along the reference: Phi_k = expm(h J(t_k+h/2, (y_k+y_{k+1})/2)) on the
    101-point grid (exponential-midpoint product; order-of-magnitude estimate).  G2 = max_{i<j} ||Phi(t_j,t_i)||_2;
    Gw = same in the case-weight norm (w = 0.01 + |ref|, i.e. per-unit-tolerance local error -> global error in
    tolerance units; rtol-independent)."""
    from scipy.linalg import expm
    out = {}
    for fam in cv.CALIBRATION_FAMILIES:
        p = cv.build(fam, n)
        r = p.reference(); T = p.output_times; S = r["states"]
        steps = []
        for k in range(len(T) - 1):
            h = T[k + 1] - T[k]
            steps.append(expm(h * p.J(T[k] + 0.5 * h, 0.5 * (S[k] + S[k + 1]))))
        W = 0.01 + np.abs(S)
        G2 = Gw = 0.0; arg2 = argw = None
        Gw_end = 0.0
        for i in range(len(T) - 1):
            P = np.eye(p.n)
            for j in range(i + 1, len(T)):
                P = steps[j - 1] @ P
                g2 = np.linalg.norm(P, 2)
                gw = np.linalg.norm((P * W[i][None, :]) / W[j][:, None], 2)
                if g2 > G2: G2, arg2 = g2, (float(T[i]), float(T[j]))
                if gw > Gw: Gw, argw = gw, (float(T[i]), float(T[j]))
                if j == len(T) - 1:
                    Gw_end = max(Gw_end, gw)
        out[p.name] = {"G2": float(G2), "G2_at": arg2, "Gw": float(Gw), "Gw_at": argw, "Gw_to_end": float(Gw_end)}
        print(f"[d] {p.name}: G2={G2:.4g} at {arg2}  Gw={Gw:.4g} at {argw}  Gw(->tf)={Gw_end:.4g}", flush=True)
    return out


def section_c_rows():
    d = json.load(open(cv.STORED_RUST_COMPACT))
    rows = []
    for rec in d["records"]:
        a = rec["artifact"]; s = a["spec"]
        row = {"case_id": s["id"], "family": s["family"], "n": s["dimension"], "rtol": s["rtol"], "atol": s["atol"],
               "reference_uncertainty_wrms": a["reference_uncertainty_wrms"], "status": a["row"]["status"]}
        for arm in ("clipped", "dense"):
            m = a[arm]["metrics"]; c = a[arm]["counters"]; dg = a[arm]["diagnostics"]
            row[arm] = {
                "max_grid_wrms": m["max_grid_wrms"], "endpoint_wrms": m["endpoint_wrms"],
                "max_grid_case": cv.case_units(m["max_grid_wrms"], s["rtol"]),
                "endpoint_case": cv.case_units(m["endpoint_wrms"], s["rtol"]),
                "attempts": dg["attempts"], "accepted": dg["accepted_macro_steps"], "rejected": dg["rejected_macro_steps"],
                "max_accepted_err_norm": max([e for e, k in zip(dg["error_norms"], dg["failure_kinds"]) if k in (None, "none", "accepted")] or [float("nan")])
                if dg["error_norms"] else None,
                "rhs_evaluations": c["rhs_evaluations"], "ft_calls": c["ft_calls"], "jvp_vectors": c["jvp_vectors"],
                "linear_solves": c["linear_solves"], "linear_iterations": c["linear_iterations"],
                "linear_matvecs": c["linear_matvecs"], "preconditioner_apps": c["preconditioner_apps"],
                "orth_dots": c["orthogonalization_inner_products"], "orth_axpys": c["orthogonalization_vector_updates"],
                "diagnostic_matvecs": c["diagnostic_matvecs"], "direct_factorizations": c["direct_factorizations"],
            }
        row["dense_exceeds_tolerance"] = row["dense"]["max_grid_case"] > 1.0
        rows.append(row)
    return rows


def main():
    which = sys.argv[1:] or ["a", "b", "c"]
    path = os.path.join(HERE, "results.json")
    res = json.load(open(path)) if os.path.exists(path) else {}
    if "a" in which:
        res["a"] = section_a(); print("[a] done", flush=True)
    if "b" in which:
        res["b_refs"] = section_b_refs()
        res["b_rust_raw"] = section_b_rust_raw()
        res["b_e03"] = section_b_e03()
    if "c" in which:
        res["c_ops_n96"] = section_c_ops(96)
        res["rust_rows"] = section_c_rows()
    if "c2" in which:   # n = 384 / 1536 with generated references (if present)
        res["c_ops_n384"] = section_c_ops_large(384)
        res["c_ops_n1536"] = section_c_ops_large(1536)
    if "d" in which:
        res["d_amplification_n96"] = section_d_amplification(96)
    json.dump(res, open(path, "w"), indent=1, default=lambda o: o if not isinstance(o, np.generic) else o.item())
    print("WROTE", path)


if __name__ == "__main__":
    main()
