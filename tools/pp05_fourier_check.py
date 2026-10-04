#!/usr/bin/env python3
"""Gate of research node research/pp05_fourier_client_20261004 (RVJ DAG node PP05).

G1: every committed native state's error bounds the actual error (complex 1-norm, max over
the two amplitudes) against a reference of the true ODE (q = sigma sqrt(1 + |a|^2 +
2|b|^2)): mpmath odefun at 30 digits for |omega| <= 40, two scipy DOP853 runs (rtol 2e-13,
8e-14) for omega = 1e4 (not rigorous; their difference is reported). Three interior points
per step against growth (start error + start mismatch + residual).
G2: the archived source certificate (Loop 10 fourier_volterra.py, exact Fractions, its own
phase witness) evaluated on every native committed path with the state a = p(0) accepts it,
and source error <= native error <= 1.01 source error.
G3: negative controls fail closed.
Also checks the 2 pi Cody-Waite constants of the native rotation enclosure at 60 digits.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
import tempfile
import zipfile
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

ZIP = Path(__file__).resolve().parents[1] / (
    "research/rvj_integration_20261004/inputs/rvj_independent_loop11_20261004.zip")


def unhex(text: str) -> float:
    return struct.unpack(">d", bytes.fromhex(text))[0]


def fr(text: str) -> F:
    return F(unhex(text))


def check_two_pi() -> dict:
    mp.mp.dps = 60
    a = mp.mpf(unhex("401921fb54000000"))
    b = mp.mpf(unhex("3e310b4611a62633"))
    d = 2 * mp.pi - a - b
    ulp_b = mp.mpf(unhex("3e310b4611a62634")) - b
    a_bits = struct.unpack(">Q", bytes.fromhex("401921fb54000000"))[0]
    return {"d": mp.nstr(d, 6), "ulp_b": mp.nstr(ulp_b, 6),
            "ok": bool(d > 0 and d < ulp_b and (a_bits & ((1 << 26) - 1)) == 0)}


def reference_mp(omega, sigma, times):
    mp.mp.dps = 30
    g = eps = mp.mpf(1) / 8
    om = mp.mpf(omega)

    def f(t, y):
        a = mp.mpc(y[0], y[1])
        b = mp.mpc(y[2], y[3])
        q = sigma * mp.sqrt(1 + abs(a) ** 2 + 2 * abs(b) ** 2)
        c = g + eps * mp.re(mp.exp(1j * om * t) * a)
        da = 1j * q * c * mp.conj(a) * b
        db = 1j * q * c * a * a / 2
        return [mp.re(da), mp.im(da), mp.re(db), mp.im(db)]

    sol = mp.odefun(f, 0, [mp.mpf(1) / 4, 0, mp.mpf(1) / 2, 0])
    out = {}
    for t in sorted(set(times)):
        y = sol(mp.mpf(t.numerator) / t.denominator)
        out[t] = (mp.mpc(y[0], y[1]), mp.mpc(y[2], y[3]))
    return out


def reference_scipy(omega, sigma, times):
    import numpy as np
    from scipy.integrate import solve_ivp
    g = eps = 0.125

    def ff(t, v):
        a, b = v
        q = sigma * np.sqrt(1 + abs(a) ** 2 + 2 * abs(b) ** 2)
        c = g + eps * np.real(np.exp(1j * omega * t) * a)
        return np.array([1j * q * c * np.conj(a) * b, 1j * q * c * a * a / 2])

    sols = []
    for rtol in (2e-13, 8e-14):
        s = solve_ivp(ff, (0, 0.5), np.array([.25 + 0j, .5 + 0j]), method="DOP853",
                      rtol=rtol, atol=rtol / 50, dense_output=True)
        if not s.success:
            raise RuntimeError(s.message)
        sols.append(s)
    out = {}
    diff = 0.0
    for t in sorted(set(times)):
        y0, y1 = sols[0].sol(float(t)), sols[1].sol(float(t))
        diff = max(diff, float(max(abs(y0 - y1))))
        out[t] = (mp.mpc(complex(y1[0])), mp.mpc(complex(y1[1])))
    return out, diff


def path_value(path, a, tau):
    return sum((mp.mpc(unhex(re), unhex(im)) * mq(tau) ** j * mp.exp(1j * k * a * mq(tau))
                for k, j, re, im in path), mp.mpc(0))


def mq(x):
    x = F(x)
    return mp.mpf(x.numerator) / x.denominator


def err1(z, w):
    d = z - w
    return abs(mp.re(d)) + abs(mp.im(d))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cases", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    data = json.loads(args.cases.read_text())
    tmp = tempfile.mkdtemp()
    zipfile.ZipFile(ZIP).extractall(tmp)
    sys.path.insert(0, str(Path(tmp) / "inputs/loop10/code"))
    import fourier_volterra as src  # noqa: E402

    g1 = g2 = True
    rows = []
    for case in data["cases"]:
        name, run = case["case"], case["run"]
        row = {"case": name, "ok": run["ok"]}
        if not run["ok"]:
            row["error"] = run["error"]
            g1 = False
            rows.append(row)
            continue
        omega, sigma = unhex(case["omega"]), case["sigma"]
        recs = run["records"]
        times = []
        for r in recs:
            t0, h = fr(r["start"]["t"]), fr(r["trial"]["h"])
            times += [t0 + h] + [t0 + tau * h for tau in (F(1, 4), F(1, 2), F(3, 4))]
        if abs(omega) <= 40:
            ref = reference_mp(omega, sigma, times)
            row["reference"] = "mpmath odefun 30 digits"
        else:
            ref, diff = reference_scipy(omega, sigma, times)
            row["reference"] = "scipy DOP853 rtol 8e-14 (difference to 2e-13 run reported)"
            row["reference_difference"] = diff
        mp.mp.dps = 30
        worst = 0.0
        worst_interior = 0.0
        parity_min = float("inf")
        parity_max = 0.0
        for r in recs:
            tr = r["trial"]
            t0, h = fr(r["start"]["t"]), fr(tr["h"])
            bound = mp.mpf(unhex(r["state"]["error"]))
            state_a = [mp.mpc(unhex(z[0]), unhex(z[1])) for z in r["state"]["a"]]
            ra = ref[t0 + h]
            e = max(err1(state_a[0], ra[0]), err1(state_a[1], ra[1]))
            worst = max(worst, float(e / bound))
            if e > bound:
                g1 = False
            a = mp.mpf(omega) * mq(h)
            interior_bound = mp.mpf(unhex(tr["growth"])) * (
                mp.mpf(unhex(r["start"]["error"])) + mp.mpf(unhex(tr["start_mismatch"]))
                + mp.mpf(unhex(tr["residual"])))
            for tau in (F(1, 4), F(1, 2), F(3, 4)):
                rv = ref[t0 + tau * h]
                vals = [path_value(p, a, tau) for p in tr["path"]]
                ei = max(err1(vals[0], rv[0]), err1(vals[1], rv[1]))
                worst_interior = max(worst_interior, float(ei / interior_bound))
                if ei > interior_bound:
                    g1 = False
            # G2: source certificate on the native path.
            m = src.Model(F(omega), sigma=sigma)
            paths = tuple(src.EP({(k, j): src.C(fr(re), fr(im)) for k, j, re, im in p})
                          for p in tr["path"])
            st = src.State(t0, tuple(p.at_zero() for p in paths), fr(r["start"]["error"]),
                           r["start"]["generation"])
            rt = src.rotate_interval(m.omega * st.t, 144)
            mid = src.C(sum(rt["cos"]) / 2, sum(rt["sin"]) / 2)
            e0 = (rt["cos"][1] - rt["cos"][0] + rt["sin"][1] - rt["sin"][0]) / 2
            phase, tail, mode = src.phase_candidate(m.omega * h, mid, 144)
            cert = src.certificate(st, h, m, paths, phase, e0 + mid.abs1() * tail, mode)
            native = fr(tr["error"])
            if not cert["accepted"]:
                g2 = False
                continue
            ratio = float(native / cert["error"])
            parity_min = min(parity_min, ratio)
            parity_max = max(parity_max, ratio)
            if not (native >= cert["error"] and native <= F(101, 100) * cert["error"]):
                g2 = False
        row.update({"steps": len(recs), "counts": run["counts"],
                    "worst_error_over_bound": worst,
                    "worst_interior_error_over_bound": worst_interior,
                    "native_over_source_error_min": parity_min,
                    "native_over_source_error_max": parity_max,
                    "final_error_bound": unhex(recs[-1]["state"]["error"]),
                    "max_physical_error": max(unhex(r["physical_error"]) for r in recs)})
        rows.append(row)
    # G3.
    nc = data["negative_controls"]
    tol4 = 1e-8 / 4
    mp.mp.dps = 30
    ref40 = reference_mp(40.0, 1, [F(1, 8)])[F(1, 8)]
    g3_items = {}
    for name in ("independent_scalar", "wrong_phase", "cyclic_alias"):
        tr = nc[name]["trial"]
        endpoint = [mp.mpc(unhex(z[0]), unhex(z[1])) for z in tr["endpoint"]]
        err = max(err1(endpoint[0], ref40[0]), err1(endpoint[1], ref40[1]))
        bound = unhex(tr["error"]) if tr["accepted"] else float("inf")
        g3_items[name] = {"actual_error": float(err), "bound": bound,
                          "bound_encloses": bool(err <= bound),
                          "committed": nc[name]["commit_within_tol_over_4"],
                          "fail_closed": (not nc[name]["commit_within_tol_over_4"]) and bool(err <= bound)}
    stale = nc["stale_binding"]
    g3_items["stale_binding"] = {"fail_closed": all(stale.values()), **stale}
    g3_items["initial_mismatch"] = {"fail_closed": not nc["initial_mismatch"]["within_tol_over_4"],
                                    "start_mismatch": unhex(nc["initial_mismatch"]["start_mismatch"])}
    g3_items["noncontractive_slab"] = {"fail_closed": nc["noncontractive_slab_refused"]}
    g3_items["domain_b_above_one"] = {"fail_closed": not nc["domain_b_above_one"]["accepted"],
                                      "reason": nc["domain_b_above_one"]["reason"]}
    g3 = all(v["fail_closed"] for v in g3_items.values())
    two_pi = check_two_pi()
    result = {"schema": "vigilode-pp05-check-v1", "cases": rows, "negative_controls": g3_items,
              "two_pi_constants": two_pi,
              "gate": {"g1_enclosure": g1, "g2_source_parity": g2, "g3_negative_controls": g3,
                       "two_pi_split_verified": two_pi["ok"],
                       "g4": "contract test pp05_fourier_client::driver_unit_case_commits_and_binds, fmt, clippy"}}
    result["verdict"] = "PASS" if (g1 and g2 and g3 and two_pi["ok"]) else "FAIL"
    args.output.write_text(json.dumps(result, indent=1, default=str) + "\n")
    print(json.dumps(result, indent=1, default=str))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
