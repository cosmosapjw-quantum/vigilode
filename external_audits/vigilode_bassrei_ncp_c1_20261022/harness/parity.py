#!/usr/bin/env python3
"""Phase P: RHS parity of every comparator adapter against the VigilODE kernel.

    python3 -s parity.py CAMP

For every case, at the initial state and three perturbed states
(scenarios.parity_states), the RHS of each adapter is compared with the
VigilODE runner's probe RHS:

    parity = max_k |f_adapter,k - f_vigilode,k| / S_k,

S_k the reference's per-component sum of |terms| (exact equality required
where S_k = 0). Adapters: the Python references (scipy arms), the Julia
ports (julia/solver.jl --parity, Base Julia), the diffsol adapter (its
#[path] build of the same kernels; --probe). An adapter above 1e-13 makes
its arms INVALID for that case (preserved, not deleted). Also reported [H]:
the coloured-AD Jacobian of the Julia port against the Python reference
Jacobian, and (chemistry) the runner's initial state against the Python one.
Chemistry probes also write results/STATIC_<case>.json, which the Julia
model description needs.
"""
import json
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np  # noqa: E402
import scenarios as S  # noqa: E402
from common import (Deadline, child_env, install_signal_cleanup, load_stdout_json, run_child,  # noqa: E402
                    utc_now, write_json)
from runners import RUNNER_BIN, Camp, pinned  # noqa: E402


def metric(fa, fv, scale):
    fa, fv, scale = (np.asarray(x, float) for x in (fa, fv, scale))
    if fa.shape != fv.shape:
        return float("inf"), "shape"
    d = np.abs(fa - fv)
    zero = scale == 0
    if np.any(d[zero] != 0):
        return float("inf"), "nonzero where the term scale is 0"
    if not np.all(np.isfinite(fa)):
        return float("inf"), "non-finite"
    with np.errstate(divide="ignore", invalid="ignore"):
        r = np.where(zero, 0.0, d / np.where(zero, 1.0, scale))
    return float(np.max(r)), None


def vigilode_probe(camp, cid, states, core):
    """Probe RHS per state (index-keyed); chemistry also saves the static grid."""
    out, init = {}, None
    raw = camp.raw_dir("parity")
    for k, (t, y) in enumerate(states):
        kind, req = S.vigilode_probe_request(cid, t, [y])
        rp = raw / f"probe_{cid}_{k}.request.json"
        rp.write_text(json.dumps(req, allow_nan=False))
        info = run_child(pinned([camp.bin / RUNNER_BIN[kind], "--request", rp], core),
                         raw / f"probe_{cid}_{k}.json", raw / f"probe_{cid}_{k}.stderr", env=child_env(),
                         timeout=Deadline().budget(300, 15)[0] or 1)
        doc, err = load_stdout_json(info)
        if doc is None or doc.get("status") != "OK":
            raise RuntimeError(f"probe {cid} t={t}: {err or (doc or {}).get('reason')}")
        if kind == "chem":
            write_json(camp.results / f"STATIC_{cid}.json", doc["grid"])
            init = (doc.get("grid") or {}).get("initial_state")
            out[k] = np.asarray(doc["states"][0]["rhs"], float)
        else:
            out[k] = np.asarray(doc["rhs"][0], float)
    return out, init


def julia_rhs(camp, cid, states, core):
    if not camp.julia or not camp.julia_depot:
        return None, "Julia unavailable (setup HOLD: no binary or no private depot copy)"
    raw = camp.raw_dir("parity")
    res = {}
    for k, (t, y) in enumerate(states):
        op = raw / f"julia_parity_{cid}_{k}.json"
        req = {"id": f"parity_{cid}_{k}", "kind": "rhs", "model": S.model_description(cid), "t": t,
               "states": [[float(v) for v in y]], "jacobian": k == 0, "output_path": str(op)}
        rp = raw / f"julia_parity_{cid}_{k}.request.json"
        rp.write_text(json.dumps(req, allow_nan=False))
        argv = camp.julia_argv(camp.harness / "julia" / "solver.jl", "--parity", rp)
        info = run_child(pinned(argv, core), raw / f"julia_parity_{cid}_{k}.stdout",
                         raw / f"julia_parity_{cid}_{k}.stderr", env=camp.julia_env(),
                         timeout=Deadline().budget(600, 15)[0] or 1)
        doc, err = load_stdout_json({"stdout_path": str(op)}) if op.exists() else \
            (None, f"no output (exit {info['exit_code']})")
        if doc is None or doc.get("status") != "OK":
            return None, err or (doc or {}).get("reason")
        res[k] = np.asarray(doc["rhs"][0], float)
        if "jacobian" in doc:
            res["_jacobian"] = (t, y, doc["jacobian"], doc.get("ncolors"))
    return res, None


def diffsol_rhs(camp, cid, states, core):
    if not camp.diffsol.exists():
        return None, "diffsol adapter not built (HOLD)"
    raw = camp.raw_dir("parity")
    res = {}
    for k, (t, y) in enumerate(states):
        req = {"mode": "probe", "model": S.model_description(cid), "t": t, "states": [[float(v) for v in y]]}
        rp = raw / f"diffsol_parity_{cid}_{k}.request.json"
        rp.write_text(json.dumps(req, allow_nan=False))
        info = run_child(pinned([camp.diffsol, "--request", rp], core), raw / f"diffsol_parity_{cid}_{k}.json",
                         raw / f"diffsol_parity_{cid}_{k}.stderr", env=child_env(),
                         timeout=Deadline().budget(300, 15)[0] or 1)
        doc, err = load_stdout_json(info)
        if doc is None or doc.get("status") != "OK":
            return None, err or (doc or {}).get("reason")
        res[k] = np.asarray(doc["rhs"][0], float)
    return res, None


def jacobian_check(cid, t, y, jac):
    """[H] max_rc |J_julia - J_ref| / sum_c |J_ref|_rc |y_c| over the pattern (state-weighted, as P04 C2)."""
    J = S.py_jac(cid, t, np.asarray(y, float))
    J = J.toarray() if hasattr(J, "toarray") else np.asarray(J, float)
    n = J.shape[0]
    Jj = np.zeros((n, n))
    Jj[np.asarray(jac["rows"]) - 1, np.asarray(jac["cols"]) - 1] = np.asarray(jac["vals"], float)
    v = np.maximum(np.abs(np.asarray(y, float)), 1e-30)
    den = np.abs(J) @ v
    num = np.abs(Jj - J) * v[None, :]
    with np.errstate(divide="ignore", invalid="ignore"):
        r = np.where(den[:, None] > 0, num / den[:, None], np.where(num > 0, np.inf, 0.0))
    outside = int(np.count_nonzero((J != 0) & (Jj == 0) & (np.abs(J) * v[None, :] > 1e-14 * den[:, None])))
    return {"max_rel_state_weighted": float(np.max(r)), "reference_nonzeros_missing_from_pattern": outside}


def main():
    install_signal_cleanup()
    camp = Camp(sys.argv[1])
    os.environ["NCPC1_RESULTS"] = str(camp.results)
    cases = sys.argv[2].split(",") if len(sys.argv) > 2 else [c[0] for c in S.CASES]
    from concurrent.futures import ThreadPoolExecutor
    pin = os.environ.get("NCPC1_PIN", "1") == "1"
    first = int(os.environ.get("NCPC1_PARITY_CORE", "8"))
    width = int(os.environ.get("NCPC1_PARITY_WIDTH", "8"))
    out = {"started_utc": utc_now(), "threshold": S.PARITY_MAX, "cases": {}}
    def one(cid, core):
        row = {}
        try:
            states = S.parity_states(cid)
            v, init = vigilode_probe(camp, cid, states, core)
            keys = list(range(len(states)))
            scale = {k: S.term_scale(cid, t, y) for k, (t, y) in enumerate(states)}
            # each adapter separately: an adapter exception marks only that adapter NOT_EVALUATED
            adapters = {}
            for name, fn in (("python_reference",
                              lambda: ({k: S.py_rhs(cid, t, y) for k, (t, y) in enumerate(states)}, None)),
                             ("julia", lambda: julia_rhs(camp, cid, states, core)),
                             ("diffsol", lambda: diffsol_rhs(camp, cid, states, core))):
                try:
                    adapters[name] = fn()
                except Exception as e:  # noqa: BLE001
                    adapters[name] = (None, f"adapter exception: {e!r}"[:500])
            for name, (res, err) in adapters.items():
                if res is None:
                    row[name] = {"status": "NOT_EVALUATED", "reason": err, "pass": False}
                    continue
                worst, why = 0.0, None
                for k in keys:
                    m, w = metric(res[k], v[k], scale[k])
                    if m > worst:
                        worst, why = m, w
                row[name] = {"status": "OK", "parity": worst, "issue": why, "pass": bool(worst <= S.PARITY_MAX)}
                if name == "julia" and "_jacobian" in res:
                    t, y, jac, ncol = res["_jacobian"]
                    if S.case_data(cid)["n"] <= 3000:
                        row[name]["jacobian_H"] = dict(jacobian_check(cid, t, y, jac), colours=ncol)
                    else:
                        row[name]["jacobian_H"] = {"skipped": "n > 3000 (dense comparison)", "colours": ncol}
            if init is not None:
                d = S.case_data(cid)
                a, b = np.asarray(init, float), np.asarray(d["y0"], float)
                row["initial_state_runner_vs_python_H"] = float(np.max(np.abs(a - b) / np.maximum(np.abs(b), 1e-300)))
            row["states"] = len(states)
        except Exception as e:  # noqa: BLE001
            import traceback
            row["error"] = traceback.format_exc()[-2000:]
            row["vigilode_probe_failed"] = repr(e)
        print(cid, json.dumps({k: (v.get("parity"), v.get("status")) if isinstance(v, dict) else v
                               for k, v in row.items()}, default=str)[:400], flush=True)
        return cid, row
    with ThreadPoolExecutor(width) as ex:
        futs = [ex.submit(one, cid, (first + k % width) if pin else None) for k, cid in enumerate(cases)]
        for f in futs:
            cid, row = f.result()
            out["cases"][cid] = row
            write_json(camp.results / "PARITY.json", dict(out, partial=True))
    out["finished_utc"] = utc_now()
    out["partial"] = False
    write_json(camp.results / "PARITY.json", out)


if __name__ == "__main__":
    main()
