"""Scenarios, arms, requests and observables of the BASS/rei NCP C1 campaign.

Cases (scenario x grid):

- S1 nonstiff: the P02 photon operator, physical q in [-4, 2.5], outflow,
  geometry R+ (H = 0.5/(1+t), s = 0.2 (1+t)^-1.5), Thomson Gamma = 0.5,
  T = 3, outputs every 0.5.
- S2 stiff: as S1 with Gamma = 1e4 (the P02b/P03 S1 scenario).
- S3 bordered: the P03 bordered model (dynamic shear, Riccati x_e with
  Gamma = 1e4 x_e, matter stress), grid 32 x 16, T = 3.
- S4N1, S4S, S4R application: the P04 photon-gas chemistry runner on its
  (16,16,16,8) grid (n = 446), scenarios N1, S and R of P04.

Grids G2 = 32x16 and G3 = 64x32 (n = 546, 2114); G4 = 128x64 (n = 8322) for
S2 with banded, client and sparse arms only. G1 = 16x8 cases are auxiliary
(dense VODE n <= 512 coverage), reported [H].

Every solver gets the same vector atol rule as P02b/P03/P04: atol =
rtol * 1e-6 * scale (cells: C |cell|; ledgers: the total; borders: P03's;
chemistry: P04's scale vector).
"""
import math
import os
import sys
from pathlib import Path

import numpy as np

HARNESS = Path(__file__).resolve().parent
REPO = Path(os.environ.get("VIGILODE_SRC", str(HARNESS.parents[2]))).resolve()
sys.dont_write_bytecode = True
sys.path.insert(0, str(REPO / "research" / "bassrei_p02_fixed_grid_20261021" / "reference"))
sys.path.insert(0, str(REPO / "research" / "bassrei_p03_block_lowrank_schur_20261022" / "reference"))
sys.path.insert(0, str(REPO / "research" / "bassrei_p04_source_exchange_stress_20261022" / "reference"))
import lrs_reference as R  # noqa: E402
import p03_reference as P3  # noqa: E402
import p04_reference as P4  # noqa: E402

LADDER = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10]
RUNG = 1e-8           # preregistered rung for the VigilODE [C] gate
REF_RTOL = 1e-12      # reference rtol (VigilODE dense/banded LU and Radau)
B_APP = 1e-6
U_REF_MAX = 1e-7
B_SRC = {"number_per_H": 5e-12, "energy_eV_per_H": 5e-11}
PARITY_MAX = 1e-13
DENSE_MAX = 2048      # dense fast driver limit
VODE_DENSE_MAX = 512  # plan: VODE MF21/22 dense only for n <= 512
DIFFSOL_DENSE_MAX = 2200

GRIDS = {"G1": (16, 8), "G2": (32, 16), "G3": (64, 32), "G4": (128, 64)}
PHYS = (-4.0, 2.5)
OUT6 = [3.0 * k / 6 for k in range(7)]
KRYLOV = {"restart": 4, "max_iterations": 48, "rtol": 1e-12, "two_level": True}
BORDER = {"alpha": 2.0, "beta": 0.5, "kappa": 1.0, "tau": 0.1, "gamma0": 1e4, "k_times_dgamma0": 0.02,
          "y0_border": [0.15, 0.1, 0.0], "absorber": [0.5, -0.5]}

# P04 scenarios (run_p04.py constants)
P04_GRID = (16, 16, 16, 8)
P04_BASE = dict(nH0=2.4e-5, H0=3e-17, tau_m=1e14, amp=1.0, xHI0=0.6, fHeI=0.2, fHeII=0.5, T0=12000.0, Dm0=0.0,
                s0_over_H0=0.1, photons_per_H=0.05, alpha=1.5, a2=0.5)
P04_SCEN = {
    "N1": dict(P04_BASE, T=1e12),
    "S": dict(P04_BASE, nH0=2.4e-3, T=1e13),
    "R": dict(P04_BASE, nH0=5.0, T0=14000.0, tau_m=1e10, T=1e12),
}
P04_TABLE = REPO / "research" / "bassrei_p04_source_exchange_stress_20261022" / "source" / "owner_grackle_rates_129.csv"


def p04_outputs(sid):
    T = P04_SCEN[sid]["T"]
    if sid == "S":
        return [0.0] + [T * 10.0 ** (-3.5 + 0.5 * k) for k in range(8)]
    return [T * k / 8 for k in range(9)]


SCENARIOS = {
    "S1": {"family": "p02", "gamma": 0.5, "stiff": False, "T": 3.0, "outputs": OUT6},
    "S2": {"family": "p02", "gamma": 1e4, "stiff": True, "T": 3.0, "outputs": OUT6},
    "S3": {"family": "bordered", "stiff": True, "T": 3.0, "outputs": OUT6},
    "S4N1": {"family": "chem", "p04": "N1", "stiff": False, "T": P04_SCEN["N1"]["T"], "outputs": p04_outputs("N1")},
    "S4S": {"family": "chem", "p04": "S", "stiff": True, "T": P04_SCEN["S"]["T"], "outputs": p04_outputs("S")},
    "S4R": {"family": "chem", "p04": "R", "stiff": True, "T": P04_SCEN["R"]["T"], "outputs": p04_outputs("R")},
}

# (case id, scenario, grid id, auxiliary?)
CASES = [
    ("S1-G1", "S1", "G1", True), ("S1-G2", "S1", "G2", False), ("S1-G3", "S1", "G3", False),
    ("S2-G1", "S2", "G1", True), ("S2-G2", "S2", "G2", False), ("S2-G3", "S2", "G3", False),
    ("S2-G4", "S2", "G4", False),
    ("S3-G1", "S3", "G1", True), ("S3-G2", "S3", "G2", False),
    ("S4N1", "S4N1", "P04", False), ("S4S", "S4S", "P04", False), ("S4R", "S4R", "P04", False),
]
CASE = {c[0]: {"case": c[0], "scenario": c[1], "grid": c[2], "auxiliary": c[3]} for c in CASES}


def geometry():
    return R.Geometry("power_law", hh=0.5, s0=0.2, p=1.5)


# ---------------------------------------------------------------------------
# Case construction
# ---------------------------------------------------------------------------

_CACHE = {}


def case_data(cid):
    """dict: family, n, y0, atol(rtol), scale, outputs, T, grid objects, the
    Python reference model (rhs/jac), the VigilODE base request."""
    if cid in _CACHE:
        return _CACHE[cid]
    c = CASE[cid]
    sc = SCENARIOS[c["scenario"]]
    fam = sc["family"]
    d = {"case": cid, "scenario": c["scenario"], "family": fam, "T": sc["T"], "outputs": list(sc["outputs"]),
         "stiff": sc["stiff"], "auxiliary": c["auxiliary"], "grid_id": c["grid"]}
    if fam in ("p02", "bordered"):
        nq, nm = GRIDS[c["grid"]]
        g = R.uniform_grid(PHYS[0], PHYS[1], nq, nm, "physical", "outflow")
        y0p, tot = R.initial_state(g)
        d["grid"] = g
        d["bandwidth"] = nm + 1
        base_scale = R.atol_vector(g, 1.0, tot)   # atol = rtol 1e-6 * this
        if fam == "p02":
            d["y0"] = y0p
            d["scale"] = base_scale
            d["gamma"] = sc["gamma"]
            d["model_py"] = R.Model(R.load_oracle(), g, geometry(), sc["gamma"], None, (0.0, sc["T"]))
            d["base_request"] = {"layout": g.layout_request(), "geometry": geometry().request(),
                                 "gamma": float(sc["gamma"]), "absorber": None, "drop_partial_t": False}
        else:
            ab = R.Absorber(*BORDER["absorber"])
            args = (BORDER["alpha"], BORDER["beta"], BORDER["kappa"], BORDER["tau"], BORDER["gamma0"], ab)
            d0 = P3.Bordered(g, geometry(), 1.0, *args).d_gamma(np.concatenate([y0p, [0.0, 0.0, 0.0]]))
            k = BORDER["k_times_dgamma0"] / d0
            m = P3.Bordered(g, geometry(), k, *args)
            d["y0"] = np.concatenate([y0p, BORDER["y0_border"]])
            d["scale"] = np.concatenate([base_scale, [0.15, 1.0, 0.01]])
            d["model_py"] = m
            d["k"] = k
            d["base_request"] = {"layout": g.layout_request(), "gamma": 0.0, "absorber": ab.request(),
                                 "geometry": geometry().request(),
                                 "border": {"k": k, "alpha": BORDER["alpha"], "beta": BORDER["beta"],
                                            "kappa": BORDER["kappa"], "tau": BORDER["tau"],
                                            "gamma0": BORDER["gamma0"]}}
        d["n"] = len(d["y0"])
    else:
        sid = sc["p04"]
        s = dict(P04_SCEN[sid])
        T = s.pop("T")
        p = P4.Params(**s)
        m = P4.make_model(p, P04_GRID)
        y0 = m.initial_state()
        d.update(y0=np.asarray(y0, float), scale=np.asarray(m.scale_vector(y0), float), model_py=m, params=p,
                 n=m.n, p04=sid)
        d["base_request"] = {"table_path": str(P04_TABLE), "grid": dict(zip(("n1", "n2", "n3", "nm"), P04_GRID)),
                             "params": {"H0": p.H0, "amp": p.amp, "tau_m": p.tau_m},
                             "initial": {k: getattr(p, k) for k in ("nH0", "xHI0", "fHeI", "fHeII", "T0", "Dm0",
                                                                      "s0_over_H0", "photons_per_H", "alpha", "a2")}}
    _CACHE[cid] = d
    return d


def atol_of(d, rtol):
    return [float(rtol * 1e-6 * v) for v in d["scale"]]


# ---------------------------------------------------------------------------
# Arms
# ---------------------------------------------------------------------------

# arm id -> (runner kind, extra)
VIGILODE_ARMS = {
    "vg_dense": ("fv", {"arm": "dense"}),
    "vg_banded": ("fv", {"arm": "banded"}),
    "vg_client_exact": ("block", {"arm": "client_exact"}),
    "vg_client_krylov": ("block", {"arm": "client_krylov", "krylov": KRYLOV}),
    "vg_bordered_dense": ("block", {"arm": "bordered_dense"}),
    "vg_bordered_client": ("block", {"arm": "bordered_client"}),
    "vg_chem_dense": ("chem", {"arm": "dense"}),
}
PY_ARMS = ["py_vode_adams_mf10", "py_vode_bdf_mf21", "py_vode_bdf_mf22", "py_vode_bdf_mf24", "py_vode_bdf_mf25",
           "py_dop853", "py_dop853_tight"]
JULIA_ARMS = ["jl_rodas5p_sparse", "jl_fbdf_sparse", "jl_cvode_bdf_band", "jl_cvode_bdf_klu", "jl_tsit5",
              "jl_vern7"]
DIFFSOL_ARMS = ["ds_bdf_dense", "ds_tsit45"]
FAMILY_OF_ARM = {}
for a in VIGILODE_ARMS:
    FAMILY_OF_ARM[a] = "vigilode"
for a in PY_ARMS:
    FAMILY_OF_ARM[a] = "dop853" if a.startswith("py_dop853") else "vode"
for a in JULIA_ARMS:
    FAMILY_OF_ARM[a] = "julia"
for a in DIFFSOL_ARMS:
    FAMILY_OF_ARM[a] = "diffsol"
FAMILY_OF_ARM["py_radau_ref"] = "reference"


def arms_for(cid):
    """(arm, rungs, applicability note) for every planned arm of a case.
    Arms not applicable a priori are listed with rungs [] and a reason
    (they appear in the coverage table as NOT_APPLICABLE)."""
    d = case_data(cid)
    fam, n, stiff = d["family"], d["n"], d["stiff"]
    big = d.get("grid_id") == "G4"
    out = []

    def add(arm, rungs=None, why=None):
        out.append({"arm": arm, "rungs": list(LADDER if rungs is None else rungs), "not_applicable": why})

    if fam == "p02":
        add("vg_dense", [] if n > DENSE_MAX else None, f"n={n} > {DENSE_MAX}" if n > DENSE_MAX else None)
        add("vg_banded")
        add("vg_client_exact")
        add("vg_client_krylov")
    elif fam == "bordered":
        add("vg_bordered_dense")
        add("vg_bordered_client")
    else:
        add("vg_chem_dense")
    # VODE
    add("py_vode_adams_mf10", None if not stiff else [], "stiff scenario (Adams functional iteration)" if stiff else None)
    dense_ok = n <= VODE_DENSE_MAX
    for a in ("py_vode_bdf_mf21", "py_vode_bdf_mf22"):
        add(a, None if dense_ok else [], None if dense_ok else f"n={n} > {VODE_DENSE_MAX} (plan)")
    band_ok = fam in ("p02", "bordered")
    for a in ("py_vode_bdf_mf24", "py_vode_bdf_mf25"):
        add(a, None if band_ok else [], None if band_ok else "chemistry Jacobian is not banded")
    # DOP853: nonstiff ladder; stiff only two rungs (censored by timeout)
    if big:
        add("py_dop853", [], "G4: banded/client/sparse arms only (plan)")
    else:
        add("py_dop853", [1e-4, 1e-8] if stiff else None)
    # DOP853 at rtol 3e-11 with max_step 1e-3 T, after the DOP853 call of rei C15 coupled.py:329-332
    # (rtol 3e-11, max_step 0.001 in its own ln-a time); atol by the campaign rule (the source's atol is
    # for a different state vector)
    add("py_dop853_tight", [3e-11] if (not stiff and not big) else [],
        None if (not stiff and not big) else "tight DOP853 run on nonstiff cases only")
    # Julia
    add("jl_rodas5p_sparse")
    add("jl_fbdf_sparse")
    add("jl_cvode_bdf_band", None if band_ok else [], None if band_ok else "chemistry Jacobian is not banded")
    add("jl_cvode_bdf_klu")
    for a in ("jl_tsit5", "jl_vern7"):
        add(a, [] if (stiff or big) else None, "stiff or G4 case" if (stiff or big) else None)
    # diffsol
    ds_ok = n <= DIFFSOL_DENSE_MAX
    add("ds_bdf_dense", None if ds_ok else [], None if ds_ok else f"n={n} > {DIFFSOL_DENSE_MAX} (dense NalgebraLU)")
    add("ds_tsit45", [] if (stiff or big) else None, "stiff or G4 case" if (stiff or big) else None)
    if big:  # G4: banded/client/sparse arms only
        for row in out:
            if row["arm"] in ("py_vode_adams_mf10",) and not row["not_applicable"]:
                row["rungs"], row["not_applicable"] = [], "G4: banded/client/sparse arms only (plan)"
    return out


def reference_arm(cid):
    d = case_data(cid)
    if d["family"] == "p02":
        return "vg_dense" if d["n"] <= DENSE_MAX else "vg_banded"
    if d["family"] == "bordered":
        return "vg_bordered_dense"
    return "vg_chem_dense"


# ---------------------------------------------------------------------------
# Requests
# ---------------------------------------------------------------------------


def vigilode_request(cid, arm, rtol, **extra):
    d = case_data(cid)
    kind, ext = VIGILODE_ARMS[arm]
    req = dict(d["base_request"])
    req.update({"mode": "integrate", "t_span": [0.0, float(d["T"])], "output_times": [float(t) for t in d["outputs"]],
                "rtol": float(rtol), "atol": atol_of(d, rtol)})
    if d["family"] != "chem":
        req["initial_state"] = [float(v) for v in d["y0"]]
    req.update(ext)
    req.update(extra)
    return kind, req


def vigilode_probe_request(cid, t, states):
    d = case_data(cid)
    req = dict(d["base_request"])
    req.update({"mode": "probe", "t": float(t), "states": [[float(v) for v in s] for s in states]})
    if d["family"] == "chem":
        kind = "chem"
    elif d["family"] == "bordered":
        kind = "block"
    else:
        kind = "fv"
    return kind, req


def model_description(cid):
    """Everything a non-Python adapter needs to build the case's RHS."""
    d = case_data(cid)
    desc = {"case": cid, "family": d["family"], "n": d["n"], "T": d["T"], "outputs": d["outputs"],
            "y0": [float(v) for v in d["y0"]], "scale": [float(v) for v in d["scale"]]}
    desc.update(d["base_request"])
    if d["family"] in ("p02", "bordered"):
        desc["bandwidth"] = d["bandwidth"]
    else:
        static = Path(os.environ.get("NCPC1_RESULTS", "/nonexistent")) / f"STATIC_{cid}.json"
        if static.is_file():
            import json
            desc["chem_static"] = json.loads(static.read_text())
    return desc


# ---------------------------------------------------------------------------
# Observables and the B_app error
# ---------------------------------------------------------------------------


def _photon_moments(g, y):
    N = np.asarray(y, float)[g.cells]  # (nq, nm)
    w = np.exp(g.qe[:-1]) * np.expm1(g.dq) / g.dq
    n = float(N.sum())
    rho = float(np.sum(w[:, None] * N))
    D = float(np.sum(w[:, None] * g.p2_exact[None, :] * N))
    return n, rho, D


def observables(cid, t, y):
    d = case_data(cid)
    y = np.asarray(y, float)
    fam = d["family"]
    if fam == "p02":
        n, rho, D = _photon_moments(d["grid"], y)
        return {"n_gamma": n, "rho_gamma": rho, "D_over_rho": D / rho if rho != 0 else 0.0, "D_gamma": D}
    if fam == "bordered":
        g = d["grid"]
        n, rho, D = _photon_moments(g, y[:g.n])
        H = 0.5 / (1.0 + t)
        return {"n_gamma": n, "rho_gamma": rho, "D_over_rho": D / rho if rho != 0 else 0.0, "D_gamma": D,
                "x_HII": float(y[g.n + 1]), "s_over_H": float(y[g.n]) / H}
    m = d["model_py"]
    dg = m.diagnostics(t, y)
    return {"n_gamma": dg["n_gamma"], "rho_gamma": dg["E_gamma"],
            "D_over_rho": dg["D_gamma"] / dg["E_gamma"] if dg["E_gamma"] != 0 else 0.0, "D_gamma": dg["D_gamma"],
            "x_HII": dg["x_HII"], "u": float(y[m.grid.IU]), "s_over_H": dg["s_over_H"]}


def floors_of(cid, outputs, ref):
    """The B_app floors of one case, from its reference:
    n_gamma, rho_gamma: 1e-3 x the initial value; D_over_rho: 1e-3 x max over
    the evaluated outputs (t > 0) of |D/rho_ref| (with the depletion weight
    below); D_gamma: 1e-3 |D_gamma(0)| (P04 precedent); x_HII: 1e-3; u: none
    (relative); s_over_H: 1e-3 |s0/H0|."""
    o0 = observables(cid, outputs[0], ref[0])
    dor = max(abs(observables(cid, t, r)["D_over_rho"]) for t, r in zip(outputs[1:], ref[1:]))
    return {"n_gamma": 1e-3 * abs(o0["n_gamma"]), "rho_gamma": 1e-3 * abs(o0["rho_gamma"]), "D_over_rho": 1e-3 * dor,
            "D_gamma": 1e-3 * abs(o0["D_gamma"]), "x_HII": 1e-3, "u": 0.0,
            "s_over_H": 1e-3 * abs(o0.get("s_over_H", 0.0))}, o0


def bapp_error(cid, outputs, states, ref, unfloored=None):
    """Max over outputs t > 0 and observables of the floored relative errors;
    a non-finite error counts as +inf. With a dict `unfloored`, also the plain
    relative errors per observable ([H])."""
    floors, o0 = floors_of(cid, outputs, ref)
    worst, per = 0.0, {}
    for t, y, r in zip(outputs[1:], states[1:], ref[1:]):
        a, b = observables(cid, t, y), observables(cid, t, r)
        for k in b:
            den = max(abs(b[k]), floors[k])
            e = abs(a[k] - b[k]) / den if den > 0 else (0.0 if a[k] == b[k] else math.inf)
            if k == "D_over_rho":
                e *= min(1.0, b["rho_gamma"] / (1e-3 * abs(o0["rho_gamma"])))
            if not math.isfinite(e):
                e = math.inf
            per[k] = max(per.get(k, 0.0), e)
            worst = max(worst, e)
            if unfloored is not None:
                u = abs(a[k] - b[k]) / abs(b[k]) if b[k] != 0 else (0.0 if a[k] == b[k] else math.inf)
                if not math.isfinite(u):
                    u = math.inf
                unfloored[k] = max(unfloored.get(k, 0.0), u)
    return worst, per


def ledger_drift(cid, states):
    """Chemistry: B_src ledger drifts per H (P04 C3); other families: the
    relative drift of the sum of all components (P02 invariant)."""
    d = case_data(cid)
    if d["family"] == "chem":
        m = d["model_py"]
        nH = m.params.nH0
        L0 = m.ledgers(np.asarray(states[0], float))
        num_h = en_h = rel = 0.0
        for s in states[1:]:
            L = m.ledgers(np.asarray(s, float))
            for k in L0:
                rel = max(rel, abs(L[k] - L0[k]) / max(abs(L0[k]), 1e-300))
            num_h = max(num_h, abs(L["number_ledger"] - L0["number_ledger"]) / nH)
            en_h = max(en_h, abs(L["energy_ledger"] - L0["energy_ledger"]) / nH)
        return {"max_rel": rel, "number_per_H": num_h, "energy_eV_per_H": en_h,
                "b_src_pass": bool(num_h <= B_SRC["number_per_H"] and en_h <= B_SRC["energy_eV_per_H"])}
    n_ph = d["grid"].n
    s0 = math.fsum(float(v) for v in states[0][:n_ph])
    rel = max(abs(math.fsum(float(v) for v in s[:n_ph]) - s0) / abs(s0) for s in states)
    return {"invariant_rel": rel}


def validate_trajectory(times, states, outputs, n):
    """None if the trajectory has exactly the requested outputs, else a refusal string."""
    if not isinstance(states, list) or not states:
        return "empty_states"
    if times is None or len(times) != len(outputs):
        return f"time_count {None if times is None else len(times)} != {len(outputs)}"
    if any(float(a) != float(b) for a, b in zip(times, outputs)):
        return "times_not_exactly_requested"
    if len(states) != len(outputs):
        return f"state_count {len(states)} != {len(outputs)}"
    for row in states:
        if not isinstance(row, list) or len(row) != n:
            return "state_shape"
        if not all(isinstance(x, (int, float)) and math.isfinite(x) for x in row):
            return "nonfinite_or_nonnumeric"
    return None


# ---------------------------------------------------------------------------
# Python reference RHS, Jacobian and term scale (parity)
# ---------------------------------------------------------------------------


def py_rhs(cid, t, y):
    d = case_data(cid)
    m = d["model_py"]
    if d["family"] == "p02":
        return np.asarray(m.f(t, np.asarray(y, float)), float)
    return np.asarray(m.rhs(t, np.asarray(y, float)), float)


def py_jac(cid, t, y):
    """Sparse (p02) or dense (bordered, chem) Jacobian of the Python reference."""
    d = case_data(cid)
    m = d["model_py"]
    if d["family"] == "p02":
        return m.jac(t, y)
    return m.jacobian(t, np.asarray(y, float))


def term_scale(cid, t, y):
    """Per-component sum of |terms| of the RHS at (t, y)."""
    d = case_data(cid)
    m = d["model_py"]
    y = np.asarray(y, float)
    if d["family"] == "p02":
        A = m.A(t).tocsr()
        return np.asarray(abs(A) @ np.abs(y), float).ravel()
    if d["family"] == "bordered":
        return np.asarray(m.term_scale(t, y), float)
    _, ab = m.rhs_with_abs(t, y)
    return np.asarray(ab, float)


def parity_states(cid, count=3, seed=20261022):
    """The initial state and lognormal perturbations at fractions of T."""
    d = case_data(cid)
    rng = np.random.default_rng(seed + sum(map(ord, cid)))
    out = [(0.0, np.asarray(d["y0"], float))]
    for k in range(count):
        y = np.asarray(d["y0"], float).copy()
        y = y * np.exp(0.2 * rng.standard_normal(len(y)))
        if d["family"] == "bordered":
            g = d["grid"]
            y[g.n] = d["y0"][g.n] * (1 + 0.2 * rng.standard_normal())
            y[g.n + 1] = min(0.9, max(0.05, d["y0"][g.n + 1] * (1 + 0.5 * abs(rng.standard_normal()))))
            y[g.n + 2] = 0.01 * rng.standard_normal()
        if d["family"] == "chem":
            m = d["model_py"]
            g = m.grid
            T_ = rng.uniform(1.1e4, 2.2e4)
            Ne = y[g.IHII] + y[g.IHeII] + 2 * y[g.IHeIII]
            Ntot = y[g.IHI] + y[g.IHII] + y[g.IHeI] + y[g.IHeII] + y[g.IHeIII] + Ne
            y[g.IU] = 1.5 * P4.KB * T_ * Ntot
            y[g.IS] = d["y0"][g.IS] * (1 + 0.3 * rng.standard_normal())
            y[g.IDm] = 1e-2 * y[g.IU] * rng.standard_normal()
        out.append((float(d["T"]) * (k + 1) / (count + 1), y))
    return out
