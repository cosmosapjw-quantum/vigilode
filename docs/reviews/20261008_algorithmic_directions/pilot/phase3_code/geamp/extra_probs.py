"""B6 stress families declared BEFORE running them (not tuned afterwards).  EXPLORATORY.

osc-amp-<AMP>: 48 decoupled 2x2 rotation blocks, n = 96, manufactured exact solution phi, span [0, 1]:
    f(t, y) = J(t) (y - phi(t)) + phi'(t),  J_k(t) = [[a(t), w_k], [-w_k, a(t)]],
    a(t) = -1 + AMP * bump(t),  bump = (tanh((t - 0.35)/0.03) - tanh((t - 0.65)/0.03)) / 2,
    w_k = 20 + 60 k / 47,  phi_k(t) = (1 + 0.5 sin(w_k t + k), 1 + 0.5 cos(w_k t + k)).
  The error propagator is exp(int a) x rotation: OSCILLATORY transient growth ~exp(0.3 AMP - 0.3) in [0.35, 0.65]
  (AMP 14: ~49x; AMP 6: ~4.5x).  This targets the judge's named false-negative risk (res5 damps rotation:
  |(1 - g i x)^-5| = 0.896 at h w = 1) and the threshold gray zone.
Predictions declared before running: AMP 14 rows R_tot > 5 (amplifying); AMP 6 rows near the threshold.
Kill checks applied unchanged (K1, K2, K3)."""
import math
import numpy as np


class _CP:
    """minimal stand-in for corpus_v2.CorpusProblem (exact reference only)."""
    def __init__(self, name, exact, span):
        self.name = name; self.exact = exact; self.span = span
        self.output_times = np.linspace(span[0], span[1], 101)

    def exact_reference(self):
        return {"source": "exact", "times": self.output_times.copy(),
                "states": np.array([self.exact(t) for t in self.output_times]), "uncertainty_wrms": 0.0}

    reference = exact_reference

    def error_metrics(self, Y, rtol, prefer_exact=True):
        import corpus_v2 as cv
        r = self.exact_reference()
        return cv.global_error_metrics(Y, r["states"], rtol, 0.0)

    def case_id(self, rtol):
        return f"{self.name}-rtol-{rtol:g}"


def osc_amp(amp, nblk=48):
    n = 2 * nblk
    w = 20.0 + 60.0 * np.arange(nblk) / (nblk - 1)
    ph = np.arange(nblk, dtype=float)

    def bump(t):
        return 0.5 * (math.tanh((t - 0.35) / 0.03) - math.tanh((t - 0.65) / 0.03))

    def dbump(t):
        return 0.5 * ((1 - math.tanh((t - 0.35) / 0.03) ** 2) - (1 - math.tanh((t - 0.65) / 0.03) ** 2)) / 0.03

    a = lambda t: -1.0 + amp * bump(t)
    da = lambda t: amp * dbump(t)

    def phi(t):
        y = np.empty(n); y[0::2] = 1 + 0.5 * np.sin(w * t + ph); y[1::2] = 1 + 0.5 * np.cos(w * t + ph); return y

    def dphi(t):
        y = np.empty(n); y[0::2] = 0.5 * w * np.cos(w * t + ph); y[1::2] = -0.5 * w * np.sin(w * t + ph); return y

    def ddphi(t):
        y = np.empty(n); y[0::2] = -0.5 * w * w * np.sin(w * t + ph); y[1::2] = -0.5 * w * w * np.cos(w * t + ph); return y

    def Jv(t, v):
        out = a(t) * v
        out[0::2] += w * v[1::2]; out[1::2] -= w * v[0::2]
        return out

    def f(t, y):
        return Jv(t, y - phi(t)) + dphi(t)

    def J(t, y):
        M = a(t) * np.eye(n)
        i = np.arange(nblk)
        M[2 * i, 2 * i + 1] = w; M[2 * i + 1, 2 * i] = -w
        return M

    def ft(t, y):
        return da(t) * (y - phi(t)) - Jv(t, dphi(t)) + ddphi(t)

    name = f"osc-amp-{amp:g}"
    return dict(f=f, J=J, ft=ft, exact=phi, y0=phi(0.0), span=(0.0, 1.0), n=n, name=name,
                cp=_CP(name, phi, (0.0, 1.0)), rhs_flops=8.0 * n + 2 * 20.0 * nblk)
