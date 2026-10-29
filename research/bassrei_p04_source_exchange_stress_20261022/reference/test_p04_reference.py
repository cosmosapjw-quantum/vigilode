"""Tests of the independent P04 Python reference (MODEL.md).

Run:  python3 -m unittest discover -s research/bassrei_p04_source_exchange_stress_20261022/reference -p 'test_*.py'

Each test prints its measured numbers to stderr as ``[P04REF] ...`` lines.
"""
from __future__ import annotations

import ctypes
import ctypes.util
import math
import sys
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import numpy as np  # noqa: E402
from scipy.integrate import quad  # noqa: E402

import p04_reference as ref  # noqa: E402

CASE_A = ref.CaseA()


def report(msg):
    print(f"[P04REF] {msg}", file=sys.stderr, flush=True)


def rel(a, b):
    a, b = float(a), float(b)
    if a == b:
        return 0.0
    return abs(a - b) / max(abs(a), abs(b))


def random_state(model, rng, T_range=(1.15e4, 2.25e4), s_frac=0.3, t_max=1e16):
    """Admissible random state: positive photons and species, T inside the
    table, |s| <= s_frac*H (one upwind branch), signed D_m, random ledgers."""
    g = model.grid
    p = model.params
    t = float(rng.uniform(0.0, t_max))
    H = model.geom.H(t)
    y = np.zeros(model.n)
    nH = p.nH0 * float(rng.uniform(0.5, 2.0))
    nHe = nH * 0.08 * float(rng.uniform(0.5, 1.5))
    xH = float(rng.uniform(0.05, 0.95))
    fr = rng.dirichlet([1.0, 1.0, 1.0])
    y[g.IHI], y[g.IHII] = xH * nH, (1 - xH) * nH
    y[g.IHeI], y[g.IHeII], y[g.IHeIII] = (float(v) * nHe for v in fr)
    cells = [c for row in g.cells for c in row]
    y[cells] = rng.uniform(0.1, 1.0, len(cells)) * p.photons_per_H * nH / len(cells)
    y[[g.LO, g.HI] + g.absorbed] = rng.uniform(0.0, 1.0, 2 + g.nq) * 1e-3 * nH
    Ne = y[g.IHII] + y[g.IHeII] + 2 * y[g.IHeIII]
    Ntot = y[g.IHI] + y[g.IHII] + y[g.IHeI] + y[g.IHeII] + y[g.IHeIII] + Ne
    T = float(rng.uniform(*T_range))
    y[g.IU] = 1.5 * ref.KB * T * Ntot
    y[g.IDm] = float(rng.uniform(-1, 1)) * 1e-2 * y[g.IU]
    y[g.IS] = float(rng.choice([-1.0, 1.0])) * float(rng.uniform(0.05, s_frac)) * H
    y[[g.IW, g.IWm, g.IEesc, g.IRe]] = rng.uniform(-1, 1, 4) * 1e-4 * y[g.IU]
    return t, y


def thomson_dominated_state(model, rng):
    """Random state with the absorbers nearly gone (x_HI, x_HeI, x_HeII ~ 1e-9),
    so transport and Thomson dominate the photon rows."""
    g = model.grid
    t, y = random_state(model, rng)
    T = model.temperature(y)
    nH = y[g.IHI] + y[g.IHII]
    nHe = y[g.IHeI] + y[g.IHeII] + y[g.IHeIII]
    y[g.IHI], y[g.IHII] = 1e-9 * nH, (1 - 1e-9) * nH
    y[g.IHeI], y[g.IHeII], y[g.IHeIII] = 1e-9 * nHe, 1e-9 * nHe, (1 - 2e-9) * nHe
    Ne = y[g.IHII] + y[g.IHeII] + 2 * y[g.IHeIII]
    y[g.IU] = 1.5 * ref.KB * T * (nH + nHe + Ne)
    return t, y


# Scenarios exercising every block: the test scenario, a dense, fast-
# expanding one with amplified shear coupling (K_D visible), and its
# Thomson-dominated states (transport + Thomson dominate the photon rows).
STRESS_PARAMS = ref.Params(nH0=3.0, H0=2e-14, tau_m=3e13, amp=1e30, xHI0=0.6, fHeI=0.2, fHeII=0.5,
                           T0=12000.0, Dm0=0.0, s0_over_H0=0.1, photons_per_H=0.05, alpha=1.5, a2=0.5)
FAMILIES = (
    ("test", ref.TEST_PARAMS, (4, 4, 4, 4), random_state),
    ("stress", STRESS_PARAMS, (3, 5, 7, 6), random_state),
    ("thomson", STRESS_PARAMS, (3, 5, 7, 6), thomson_dominated_state),
)


class TestConstantsGridVerner(unittest.TestCase):
    def test_table_hash_and_constants(self):
        self.assertEqual(ref.sha256_file(ref.CASE_A_CSV), ref.CASE_A_SHA256)
        self.assertEqual(CASE_A.T_min, 1.00000000000000091e4)
        self.assertEqual(CASE_A.T_max, 2.40000000000000291e4)
        kd = 8 * math.pi * ref.G / (3 * ref.C * ref.C) * ref.EV_ERG
        self.assertEqual(ref.K_D, kd)
        report(f"K_D = {ref.K_D!r} cm^3 eV^-1 s^-2; T domain [{CASE_A.T_min!r}, {CASE_A.T_max!r}] K")

    def test_grid_faces_and_layout(self):
        for dims in ((4, 4, 4, 4), (3, 5, 7, 6), (1, 1, 1, 1), (16, 16, 16, 8)):
            g = ref.Grid(*dims)
            n1, n2, n3, nm = dims
            self.assertEqual(g.nq, n1 + n2 + n3)
            self.assertEqual(len(g.q_edges), g.nq + 1)
            self.assertEqual(g.q_edges[0], math.log(13.60))
            self.assertEqual(g.q_edges[n1], math.log(24.59))
            self.assertEqual(g.q_edges[n1 + n2], math.log(54.42))
            self.assertEqual(g.q_edges[-1], math.log(100.0))
            self.assertTrue(all(b > a for a, b in zip(g.q_edges, g.q_edges[1:])))
            self.assertEqual(g.mu_edges[0], -1.0)
            self.assertEqual(g.mu_edges[-1], 1.0)
            self.assertEqual(g.n_ph, 2 + g.nq * (nm + 1))
            self.assertEqual(g.n, g.n_ph + 12)
            self.assertEqual((g.IW, g.IS), (g.n_ph, g.n_ph + 11))
            idx = sorted([g.LO, g.HI] + [c for r in g.cells for c in r] + g.absorbed)
            self.assertEqual(idx, list(range(g.n_ph)))
            # exact cell averages: sum dm_j/2 = 1, sum dm_j p_j = 0, c_j = 2 p_j
            self.assertAlmostEqual(math.fsum(d / 2 for d in g.dm), 1.0, places=15)
            self.assertLess(abs(math.fsum(d * pj for d, pj in zip(g.dm, g.p))), 1e-15)
            self.assertLess(max(abs(cj - 2 * pj) for cj, pj in zip(g.c, g.p)), 4e-16)
            # sbar zero below thresholds, positive above
            for s, chi in enumerate((13.60, 24.59, 54.42)):
                for i in range(g.nq):
                    below = g.q_edges[i + 1] <= math.log(chi)
                    self.assertEqual(g.sbar[s][i] == 0.0, below)

    def test_verner_vector_equals_scalar_and_thresholds(self):
        rng = np.random.default_rng(7)
        E = np.concatenate([np.exp(rng.uniform(math.log(10.0), math.log(120.0), 20000)),
                            [13.60, 24.59, 54.42, np.nextafter(13.60, 0), 100.0]])
        diff = 0
        for s in range(3):
            vec = ref.verner_sigma(s, E)
            for e, v in zip(E.tolist(), vec.tolist()):
                if v != ref.verner_sigma_scalar(s, e):
                    diff += 1
            self.assertEqual(ref.verner_sigma_scalar(s, np.nextafter(ref.FIT[s][0], 0)), 0.0)
            self.assertGreater(ref.verner_sigma_scalar(s, ref.FIT[s][0]), 0.0)
        report(f"Verner vectorized vs scalar: {3 * E.size} values, {diff} differ")
        self.assertEqual(diff, 0)
        # Sanity: sigma_HI(13.6 eV) ~ 6.3e-18 cm^2 (Verner 1996 HI threshold value)
        self.assertAlmostEqual(ref.verner_sigma_scalar(0, 13.60) / 6.3e-18, 1.0, delta=0.02)

    def test_hypot_is_libm(self):
        name = ctypes.util.find_library("m")
        if not name:
            self.skipTest("libm not found")
        lib = ctypes.CDLL(name)
        lib.hypot.restype = ctypes.c_double
        lib.hypot.argtypes = [ctypes.c_double, ctypes.c_double]
        g = ref.Grid(16, 16, 16, 4)
        xs, _ = ref.gauss_legendre()
        bad = 0
        tot = 0
        for i in range(g.nq):
            lo, hi = g.q_edges[i], g.q_edges[i + 1]
            for xk in xs:
                E = math.exp(0.5 * (lo + hi) + (0.5 * (hi - lo)) * xk)
                x = E / ref.FIT[1][1] - ref.FIT[1][6]
                tot += 1
                bad += float(np.hypot(x, ref.FIT[1][7])) != lib.hypot(x, ref.FIT[1][7])
        report(f"numpy.hypot vs ctypes libm hypot on {tot} HeI GL nodes: {bad} differ")
        self.assertEqual(bad, 0)

    def test_sbar_scalar_vector_and_quadrature_error(self):
        g = ref.Grid(4, 4, 4, 4)
        worst_vs_vec = 0.0
        worst_quad = 0.0
        for s in range(3):
            for i in range(g.nq):
                lo, hi = g.q_edges[i], g.q_edges[i + 1]
                a = g.sbar[s][i]
                b = ref.cell_average_sigma(s, lo, hi, scalar=False)
                worst_vs_vec = max(worst_vs_vec, rel(a, b))
                if a > 0:
                    exact = quad(lambda q: ref.verner_sigma_scalar(s, math.exp(q)), lo, hi,
                                 epsabs=0, epsrel=1e-13, limit=200)[0] / (hi - lo)
                    worst_quad = max(worst_quad, rel(a, exact))
        report(f"sbar scalar vs vectorized max rel {worst_vs_vec:.3e}; 16-pt GL vs adaptive quad max rel "
               f"{worst_quad:.3e} (grid 4,4,4)")
        self.assertEqual(worst_vs_vec, 0.0)
        self.assertLess(worst_quad, 1e-12)

    def test_geometry_closed_forms(self):
        geo = ref.Geometry(3e-17)
        for t in (0.0, 1e12, 3e15, 4e16):
            # absolute: log(a) of a = (1+x)^(2/3) carries the 1+x rounding (eps/2 absolute)
            self.assertLessEqual(abs(geo.ln_a(t) - math.log(geo.a(t))), 4e-16)
            self.assertLess(rel(geo.a_m3(t), geo.a(t) ** -3), 1e-14)
            d = 1e-4 * (geo.t0 + t)
            fd_H = (geo.H(t + d) - geo.H(t - d)) / (2 * d)
            fd_a = (geo.a_m3(t + d) - geo.a_m3(t - d)) / (2 * d)
            self.assertLess(rel(fd_H, geo.dH(t)), 1e-7)
            self.assertLess(rel(fd_a, geo.da_m3(t)), 1e-7)
            self.assertLess(rel(geo.H(t), (2 / 3) / (geo.t0 + t)), 1e-16)
        self.assertLess(rel(geo.H(0.0), 3e-17), 1e-15)


class TestPchip(unittest.TestCase):
    """(a) manual PCHIP == scipy PchipInterpolator, values and derivatives."""

    def test_knots_from_numpy_log_equal_math_log(self):
        self.assertEqual(CASE_A.x, CASE_A.x_np.tolist())
        self.assertEqual(CASE_A.ylog, CASE_A.y_np.T.tolist())

    def test_coefficients_bitwise(self):
        c = CASE_A.pp.c  # (4, 128, 8)
        diff = 0
        for col in range(8):
            for k in range(128):
                for r in range(4):
                    diff += CASE_A.manual.coef[col][k][r] != float(c[r, k, col])
        report(f"PCHIP Hermite coefficients manual vs scipy: {4 * 128 * 8} entries, {diff} differ")
        self.assertEqual(diff, 0)

    def _compare(self, X):
        scipy_P = CASE_A.pp(np.asarray(X))
        scipy_dP = CASE_A.dpp(np.asarray(X))
        scipy_dP_nu = CASE_A.pp(np.asarray(X), 1)
        n_val = n_bit = n_der_bit = n_nu_bit = 0
        max_rel = max_rel_d = max_rel_nu = 0.0
        for a, Xa in enumerate(X):
            vals, ders = CASE_A.manual.eval(Xa, deriv=True)
            for col in range(8):
                n_val += 1
                sv, sd, sn = float(scipy_P[a, col]), float(scipy_dP[a, col]), float(scipy_dP_nu[a, col])
                n_bit += vals[col] == sv
                n_der_bit += ders[col] == sd
                n_nu_bit += ders[col] == sn
                max_rel = max(max_rel, rel(vals[col], sv))
                max_rel_d = max(max_rel_d, rel(ders[col], sd))
                max_rel_nu = max(max_rel_nu, rel(ders[col], sn))
        return n_val, n_bit, max_rel, n_der_bit, max_rel_d, n_nu_bit, max_rel_nu

    def test_nodes(self):
        r = self._compare(CASE_A.x)
        report(f"PCHIP at 129 nodes x 8 cols: {r[0]} values, bitwise {r[1]}, max rel {r[2]:.3e}; "
               f"P' vs derivative(): bitwise {r[3]}, max rel {r[4]:.3e}; P' vs __call__(nu=1): bitwise {r[5]}, "
               f"max rel {r[6]:.3e}")
        self.assertLessEqual(r[2], 1e-15)
        self.assertLessEqual(r[4], 1e-15)
        # Nodes 0..127 reproduce ln v exactly (s = 0); the last node is
        # evaluated on interval m-2 with s = h_{m-2}, so it is within roundoff.
        for k, X in enumerate(CASE_A.x):
            vals = CASE_A.manual.eval(X)
            for col in range(8):
                if k < 128:
                    self.assertEqual(vals[col], CASE_A.ylog[col][k])
                else:
                    self.assertLessEqual(abs(vals[col] - CASE_A.ylog[col][k]),
                                         4 * math.ulp(CASE_A.ylog[col][k]))

    def test_dense_sweep(self):
        lo, hi = CASE_A.T_min, CASE_A.T_max
        lt = np.linspace(math.log(lo), math.log(hi), 20001)
        T = np.exp(lt)
        T[0], T[-1] = lo, hi
        T = np.clip(T, lo, hi)
        X = [math.log(float(v)) for v in T]
        r = self._compare(X)
        report(f"PCHIP dense sweep 20001 T x 8 cols: {r[0]} values, bitwise {r[1]}, max rel {r[2]:.3e}; "
               f"P' vs derivative(): bitwise {r[3]}, max rel {r[4]:.3e}; P' vs __call__(nu=1): bitwise {r[5]}, "
               f"max rel {r[6]:.3e}")
        self.assertLessEqual(r[2], 1e-15)
        self.assertLessEqual(r[4], 1e-15)
        self.assertLessEqual(r[6], 1e-15)
        # Coefficients exp(P) through CaseA, both backends.
        nbit = 0
        mx = 0.0
        mxd = 0.0
        for Tv in T.tolist():
            a, da = CASE_A.rates_and_dT(Tv, "scipy")
            b, db = CASE_A.rates_and_dT(Tv, "manual")
            nbit += sum(x == y for x, y in zip(a, b))
            mx = max(mx, max(rel(x, y) for x, y in zip(a, b)))
            mxd = max(mxd, max(rel(x, y) for x, y in zip(da, db)))
        report(f"Case-A coefficients exp(P) scipy vs manual: {8 * T.size} values, bitwise {nbit}, max rel "
               f"{mx:.3e}; dv/dT max rel {mxd:.3e}")
        self.assertLessEqual(mx, 1e-15)
        self.assertLessEqual(mxd, 1e-15)

    def test_derivative_against_fd(self):
        # P' of the spec against central FD of the spline inside intervals.
        worst = 0.0
        for k in range(0, 128, 7):
            X = 0.5 * (CASE_A.x[k] + CASE_A.x[k + 1])
            h = 1e-5 * (CASE_A.x[k + 1] - CASE_A.x[k])
            _, d = CASE_A.manual.eval(X, deriv=True)
            vp, vm = CASE_A.manual.eval(X + h), CASE_A.manual.eval(X - h)
            for col in range(8):
                fd = (vp[col] - vm[col]) / (2 * h)
                worst = max(worst, abs(fd - d[col]) / max(abs(d[col]), 1e-3))
        self.assertLess(worst, 1e-6)


class TestDomainRefusal(unittest.TestCase):
    """(b) strict inclusive domain; refusal outside, CASE_A_T_DOMAIN prefix."""

    def test_case_a_refusal(self):
        for T in (CASE_A.T_min * (1 - 1e-15), CASE_A.T_max * (1 + 1e-15), float("nan"), float("inf"),
                  -float("inf"), 0.0, np.nextafter(CASE_A.T_min, 0), np.nextafter(CASE_A.T_max, np.inf)):
            for backend in ("scipy", "manual"):
                with self.assertRaises(ref.RateDomainError) as cm:
                    CASE_A.rates(float(T), backend)
                self.assertTrue(str(cm.exception).startswith("CASE_A_T_DOMAIN:"))
        self.assertLess(CASE_A.T_min * (1 - 1e-15), CASE_A.T_min)
        self.assertGreater(CASE_A.T_max * (1 + 1e-15), CASE_A.T_max)
        # Inclusive endpoints evaluate to the table values to the exp(log v)
        # round trip, |ln v| * eps (|ln v| <= 92 here).
        for T, k in ((CASE_A.T_min, 0), (CASE_A.T_max, 128)):
            for backend in ("scipy", "manual"):
                v = CASE_A.rates(T, backend)
                for col in range(8):
                    lnv = abs(math.log(CASE_A.values[col][k]))
                    self.assertLessEqual(rel(v[col], CASE_A.values[col][k]), 1.5 * lnv * 2.0 ** -52)
        report("refusal at T_min*(1-1e-15), T_max*(1+1e-15), nextafter, NaN, +-inf, 0: all CASE_A_T_DOMAIN")

    def test_model_refusal_counts(self):
        m = ref.make_model()
        y = m.initial_state()
        g = m.grid
        for target, side in ((CASE_A.T_min * (1 - 1e-15), "low"), (CASE_A.T_max * (1 + 1e-15), "high")):
            yy = y.copy()
            Ne = yy[g.IHII] + yy[g.IHeII] + 2 * yy[g.IHeIII]
            Ntot = yy[g.IHI] + yy[g.IHII] + yy[g.IHeI] + yy[g.IHeII] + yy[g.IHeIII] + Ne
            yy[g.IU] = 1.5 * ref.KB * target * Ntot
            T = m.temperature(yy)
            self.assertTrue(T < CASE_A.T_min if side == "low" else T > CASE_A.T_max)
            before = m.domain_refusals
            for fn in (m.rhs, m.jacobian, m.partial_t):
                with self.assertRaises(ref.RateDomainError) as cm:
                    fn(0.0, yy)
                self.assertTrue(str(cm.exception).startswith("CASE_A_T_DOMAIN:"))
            self.assertEqual(m.domain_refusals, before + 3)
        yy = y.copy()
        yy[g.IU] = float("nan")
        with self.assertRaises(ref.RateDomainError):
            m.rhs(0.0, yy)
        self.assertEqual(m.domain_refusals, 7)

    def test_radau_failure_record(self):
        # A hot, photon-rich start leaves the table through photoheating.
        p = ref.Params(nH0=2.4e-5, H0=3e-17, tau_m=1e14, T0=2.3e4, xHI0=0.9, photons_per_H=0.5)
        m = ref.make_model(p, (4, 4, 4, 4))
        y0 = m.initial_state()
        res = ref.integrate_radau((0.0, 1e13), y0, None, 1e-8, 1e-14 * m.scale_vector(y0), m)
        report(f"out-of-domain Radau run: success={res['success']} status={res['status']} "
               f"refusals={res['domain_refusals']} failure={res['failure']}")
        self.assertFalse(res["success"])
        self.assertEqual(res["failure"]["kind"], "NonFinite")
        self.assertTrue(res["failure"]["message"].startswith("CASE_A_T_DOMAIN:"))
        self.assertGreater(res["domain_refusals"], 0)
        self.assertEqual(m.domain_refusals, res["domain_refusals"])


class TestInvariants(unittest.TestCase):
    """(c) weighted sums of the RHS vanish to roundoff."""

    def test_weighted_rhs_sums(self):
        rng = np.random.default_rng(20261022)
        worst = {}
        nstates = 0
        for params, grid, count, make_state in ((ref.TEST_PARAMS, (4, 4, 4, 4), 40, random_state),
                                                (STRESS_PARAMS, (3, 5, 7, 6), 30, random_state),
                                                (STRESS_PARAMS, (3, 5, 7, 6), 20, thomson_dominated_state),
                                                (ref.TEST_PARAMS, (16, 16, 16, 8), 4, random_state)):
            m = ref.make_model(params, grid)
            W = m.invariant_weights()
            for _ in range(count):
                t, y = make_state(m, rng)
                nstates += 1
                f, a = m.rhs_with_abs(t, y)
                np.testing.assert_array_equal(f, m.rhs(t, y))
                for name, w in W.items():
                    s = math.fsum((w * f).tolist())
                    scale = math.fsum((np.abs(w) * a).tolist())
                    ratio = abs(s) / scale
                    worst[name] = max(worst.get(name, 0.0), ratio)
                    self.assertLessEqual(ratio, 1e-14, f"{name}: {ratio}")
        report(f"invariant weighted-RHS |sum w f| / sum |w| |terms| ({nstates} states): "
               + ", ".join(f"{k} {v:.2e}" for k, v in worst.items()))

    def test_thomson_and_transport_parts(self):
        # The ledgers also hold for the bare initial state of the test scenario.
        m = ref.make_model()
        y = m.initial_state()
        f, a = m.rhs_with_abs(0.0, y)
        for name, w in m.invariant_weights().items():
            s = math.fsum((w * f).tolist())
            self.assertLessEqual(abs(s), 1e-14 * math.fsum((np.abs(w) * a).tolist()))


def jac_steps(model, y):
    """FD steps: f is exactly linear in photon slots, ledgers, D_m and (on one
    upwind branch) s; nonlinear (through T and the rates) in species and U.
    Nonlinear steps start at 1e-4 relative (species: at least 1e-6 of the
    total particle density, so a trace species is not differenced with a
    step below the roundoff of its rows) and are quartered until the whole
    Richardson stencil lies in one PCHIP interval (the spline is only C1 at
    its knots, where a central difference loses its h^2 error expansion)."""
    g = model.grid
    sc = model.scale_vector(y)
    h = np.maximum(np.abs(y), sc)
    Ne = y[g.IHII] + y[g.IHeII] + 2 * y[g.IHeIII]
    Ntot = y[g.IHI] + y[g.IHII] + y[g.IHeI] + y[g.IHeII] + y[g.IHeIII] + Ne
    for k in (g.IHI, g.IHII, g.IHeI, g.IHeII, g.IHeIII, g.IU):
        hk = 1e-4 * abs(y[k]) if k == g.IU else max(1e-4 * abs(y[k]), 1e-6 * Ntot)
        for _ in range(20):
            ints = set()
            for d in (0.0, hk, -hk, 0.5 * hk, -0.5 * hk):
                yy = y.copy()
                yy[k] = y[k] + d
                ints.add(CASE_A.manual.interval(math.log(model.temperature(yy))))
            if len(ints) == 1:
                break
            hk *= 0.25
        h[k] = hk
    h[g.IS] = 1e-3 * abs(y[g.IS])
    return h


class TestJacobian(unittest.TestCase):
    """(d) analytic Jacobian vs central Richardson FD, per column."""

    def test_jacobian_vs_fd(self):
        rng = np.random.default_rng(4)
        worst_col = {}
        worst_entry = {}
        nstates = 0
        for fam, params, grid, make_state in FAMILIES:
            m = ref.make_model(params, grid)
            names = m.grid.index_names()
            for _ in range(4):
                t, y = make_state(m, rng)
                J = m.jacobian(t, y)
                Jfd = ref.finite_difference_jacobian(m.rhs, t, y, jac_steps(m, y))
                nstates += 1
                for k in range(m.n):
                    col, fd = J[:, k], Jfd[:, k]
                    cm = np.max(np.abs(col))
                    if cm == 0.0:
                        self.assertEqual(np.max(np.abs(fd)), 0.0, f"column {names[k]} should be zero")
                        continue
                    e = np.max(np.abs(col - fd)) / cm
                    worst_col[fam] = max(worst_col.get(fam, 0.0), e)
                    self.assertLessEqual(e, 1e-6, f"{fam} column {names[k]}: {e}")
                    sig = np.abs(col) >= 1e-3 * cm   # well above the FD roundoff floor
                    worst_entry[fam] = max(worst_entry.get(fam, 0.0),
                                           float(np.max(np.abs(col - fd)[sig] / np.abs(col[sig]))))
        report(f"Jacobian vs FD ({nstates} states): max per-column rel "
               + ", ".join(f"{k} {v:.2e}" for k, v in worst_col.items())
               + "; max per-entry rel (entries >= 1e-3 col max) "
               + ", ".join(f"{k} {v:.2e}" for k, v in worst_entry.items()))
        for v in worst_entry.values():
            self.assertLessEqual(v, 1e-6)

    def test_photon_rows_exactly_linear_in_species(self):
        """Photon and W rows are exactly linear in every species (Gam in N_e,
        k_s in N_s; T does not enter): a 25% step pins those columns (the
        Thomson dGam/dN entries sit below the per-column FD threshold). The
        tolerance is 1e-9 relative plus the accumulation roundoff floor
        8*eps*(sum of |terms| of the row)/step."""
        rng = np.random.default_rng(13)
        eps = 2.0 ** -52
        worst_ratio = 0.0
        worst_thomson = 0.0
        for fam, params, grid, make_state in FAMILIES:
            m = ref.make_model(params, grid)
            g = m.grid
            rows = list(range(g.n_ph + 1))
            for _ in range(4):
                t, y = make_state(m, rng)
                J = m.jacobian(t, y)
                f0, ab = m.rhs_with_abs(t, y)
                for k in (g.IHI, g.IHII, g.IHeI, g.IHeII, g.IHeIII):
                    yp = y.copy()
                    yp[k] += 0.25 * y[k]
                    h = yp[k] - y[k]
                    d = (m.rhs(t, yp) - f0)[rows] / h
                    col = J[rows, k]
                    bound = 1e-9 * np.abs(col) + 8 * eps * ab[rows] / h
                    worst_ratio = max(worst_ratio, float(np.max(np.abs(d - col) / bound)))
                    if fam == "thomson" and k in (g.IHII, g.IHeIII):
                        nz = col != 0.0
                        worst_thomson = max(worst_thomson, float(np.max(np.abs(d - col)[nz] / np.abs(col[nz]))))
        report(f"photon/W rows vs species (exact linearity): max |d-J|/(1e-9|J| + roundoff floor) "
               f"{worst_ratio:.2e}; Thomson-dominated dGam/dN_HII, dN_HeIII entries max rel {worst_thomson:.2e}")
        self.assertLessEqual(worst_ratio, 1.0)
        self.assertLessEqual(worst_thomson, 1e-9)

    def test_transport_columns_equal_unit_vector_transport(self):
        m = ref.make_model(STRESS_PARAMS, (3, 5, 7, 6))
        rng = np.random.default_rng(9)
        t, y = random_state(m, rng)
        g = m.grid
        H = m.geom.H(t)
        s = y[g.IS]
        Jt = np.zeros((g.n_ph, m.n))
        m._transport_jacobian(H, s, Jt)
        for row in g.cells:
            for c in row:
                out = [0.0] * m.n
                e = [0.0] * m.n
                e[c] = 1.0

                def add(k, v):
                    out[k] += v
                m._transport(H, s, H, s, e, add)
                np.testing.assert_array_equal(Jt[:, c], np.array(out[: g.n_ph]))


class TestPartialT(unittest.TestCase):
    """(e) analytic f_t vs central Richardson FD in t."""

    def test_partial_t_vs_fd(self):
        rng = np.random.default_rng(11)
        worst = 0.0
        worst_comp = 0.0
        nstates = 0
        for fam, params, grid, make_state in FAMILIES:
            m = ref.make_model(params, grid)
            for _ in range(5):
                t, y = make_state(m, rng)
                ft = m.partial_t(t, y)
                fd = ref.finite_difference_partial_t(m.rhs, t, y, 1e-4 * (m.geom.t0 + t))
                nstates += 1
                e = np.max(np.abs(ft - fd)) / np.max(np.abs(ft))
                worst = max(worst, e)
                self.assertLessEqual(e, 1e-6)
                sig = np.abs(ft) >= 1e-6 * np.max(np.abs(ft))
                worst_comp = max(worst_comp, float(np.max(np.abs(ft - fd)[sig] / np.abs(ft[sig]))))
        report(f"partial_t vs FD ({nstates} states): max rel (max-norm) {worst:.2e}; max per-component rel "
               f"(components >= 1e-6 max) {worst_comp:.2e}")
        self.assertLessEqual(worst_comp, 1e-6)

    def test_shear_row_needs_a_m3_term(self):
        """MODEL.md section 8's bullet list names only dH/dt for the s row, but
        the s row's K_D term contains a^-3(t). With amp = 1e30 the term
        dominates f_t[s]; FD confirms it (the bullet-list-only value fails)."""
        m = ref.make_model(STRESS_PARAMS, (3, 5, 7, 6))
        g = m.grid
        rng = np.random.default_rng(17)
        worst_full = 0.0
        worst_bullet = np.inf
        for _ in range(5):
            t, y = random_state(m, rng)
            ft = m.partial_t(t, y)
            fd = ref.finite_difference_partial_t(m.rhs, t, y, 1e-4 * (m.geom.t0 + t))
            bullet_only = -3 * m.geom.dH(t) * y[g.IS]
            worst_full = max(worst_full, rel(ft[g.IS], fd[g.IS]))
            worst_bullet = min(worst_bullet, rel(bullet_only, fd[g.IS]))
        report(f"f_t[s]: with the K_D*d(a^-3)/dt term max rel vs FD {worst_full:.2e}; dH/dt-only "
               f"(spec bullet list) min rel vs FD {worst_bullet:.2e} (amp=1e30 stress scenario)")
        self.assertLessEqual(worst_full, 1e-6)
        self.assertGreater(worst_bullet, 0.5)

    def test_partial_t_at_t0(self):
        m = ref.make_model()
        y = m.initial_state()
        ft = m.partial_t(0.0, y)
        fd = ref.finite_difference_partial_t(m.rhs, 0.0, y, 1e-4 * m.geom.t0)
        self.assertLessEqual(np.max(np.abs(ft - fd)) / np.max(np.abs(ft)), 1e-6)


class TestPhotoionizationEnergy(unittest.TestCase):
    """(f) photon energy loss = BIND*events + heat, per channel."""

    def test_energy_identity(self):
        rng = np.random.default_rng(5)
        worst = 0.0
        for params, grid in ((ref.TEST_PARAMS, (4, 4, 4, 4)), (ref.TEST_PARAMS, (16, 16, 16, 8))):
            m = ref.make_model(params, grid)
            g = m.grid
            for _ in range(10):
                t, y = random_state(m, rng)
                ev = m.photoionization_events(t, y)
                for s in range(3):
                    loss = math.fsum(E * R for _, _, R, E in ev[s])
                    events = math.fsum(R for _, _, R, _ in ev[s])
                    heat = math.fsum((E - ref.BIND[s]) * R for _, _, R, E in ev[s])
                    e = abs(loss - (ref.BIND[s] * events + heat)) / loss
                    worst = max(worst, e)
                    self.assertLessEqual(e, 1e-15)
                    self.assertGreater(heat, 0.0)
                # The RHS uses the identical float R: A_i is a pure photoionization accumulator.
                f = m.rhs(t, y)
                for i in range(g.nq):
                    acc = 0.0
                    for s in range(3):
                        for ii, _, R, _ in ev[s]:
                            if ii == i:
                                acc += R
                    self.assertEqual(f[g.absorbed[i]], acc)
        report(f"photoionization energy identity: max rel {worst:.2e}; A_i bitwise = sum of R (s, j order)")


class TestRadau(unittest.TestCase):
    """(g) short Radau run of the test scenario; invariant drift."""

    def test_short_integration(self):
        m, y0, res = ref.test_scenario_run()
        self.assertTrue(res["success"], res["message"])
        self.assertEqual(res["domain_refusals"], 0)
        L0 = m.ledgers(y0)
        drift = {}
        for row in res["y"]:
            L = m.ledgers(row)
            for k, v in L.items():
                drift[k] = max(drift.get(k, 0.0), abs(v - L0[k]) / abs(L0[k]))
        d0 = m.diagnostics(0.0, y0)
        d1 = m.diagnostics(res["t"][-1], res["y"][-1])
        report(f"Radau t=[0,1e12] rtol 1e-10 atol 1e-14*scale: nfev {res['nfev']} njev {res['njev']} "
               f"nlu {res['nlu']}; drift " + ", ".join(f"{k} {v:.2e}" for k, v in drift.items()))
        report("diagnostics t=0: " + ", ".join(f"{k}={v:.10g}" for k, v in d0.items()))
        report("diagnostics t=1e12: " + ", ".join(f"{k}={v:.10g}" for k, v in d1.items()))
        for k, v in drift.items():
            self.assertLessEqual(v, 1e-10, k)
        self.assertGreater(d1["x_HII"], d0["x_HII"])
        self.assertLess(d1["n_gamma"], d0["n_gamma"])
        self.assertTrue(CASE_A.T_min <= d1["T"] <= CASE_A.T_max)


if __name__ == "__main__":
    unittest.main()
