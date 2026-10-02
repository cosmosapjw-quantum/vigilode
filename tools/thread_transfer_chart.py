#!/usr/bin/env python3
"""Optional Darboux/closure chart contract (research node research/thread_transfer_chart_contract_20261002,
thread-transfer DAG node P2-CHART-CONTRACT). Research module; no solver uses it.

For an ODE z' = F(z) and functions C, D with

    L_F C = (-kappa + a) C + r_C,    L_F D = a D + r_D,

the ratio w = C / D satisfies w' = -kappa w + (r_C - w r_D) / D. Small cofactor residuals therefore do
not make the chart accurate where |D| is small, and the chart changes the state, its reconstruction,
the initial condition and the branch. `SemilinearChart` states those conditions for the semilinear
model x' = x^2, y' = (-kappa + 2x) y + x^2 (C = y - x^2 / kappa, D = x^2, w = y / x^2 - 1 / kappa) and
refuses to map where they fail. Each check below is executed and recorded; the script exits nonzero if
one fails.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import mpmath as mp
import sympy as sp

SCHEMA = "vigilode-thread-transfer-chart-contract-v1"


class ChartDomainError(ValueError):
    """The chart is not valid at this point (D too small, ill-conditioned, or another branch)."""


def ratio_chart_identity() -> dict:
    t = sp.Symbol("t")
    kappa = sp.Symbol("kappa", positive=True)
    C, D, a, rC, rD = (sp.Function(name)(t) for name in ("C", "D", "a", "r_C", "r_D"))
    w = C / D
    derivative = sp.diff(w, t).subs({
        sp.Derivative(C, t): (-kappa + a) * C + rC,
        sp.Derivative(D, t): a * D + rD,
    })
    general = sp.simplify(derivative - (-kappa * w + (rC - w * rD) / D)) == 0
    # The semilinear chart: both residuals vanish identically.
    x, y = sp.symbols("x y")
    F = sp.Matrix([x**2, (-kappa + 2 * x) * y + x**2])
    Cx = y - x**2 / kappa
    Dx = x**2
    lie = lambda g: (sp.Matrix([g]).jacobian([x, y]) * F)[0]
    a_x = sp.simplify(lie(Dx) / Dx)
    r_D = sp.simplify(lie(Dx) - a_x * Dx)
    r_C = sp.simplify(lie(Cx) - (-kappa + a_x) * Cx)
    return {"general_identity": bool(general), "semilinear_a": str(a_x), "semilinear_r_C": str(r_C),
            "semilinear_r_D": str(r_D), "semilinear_exact": r_C == 0 and r_D == 0}


def moving_frame_connection() -> dict:
    x, kappa = sp.symbols("x kappa")
    F_x = x**2
    S = x**2
    connection = sp.simplify(-sp.diff(S, x) * F_x / S)
    return {"frame": "S = x^2", "connection": str(connection), "equals_minus_2x": sp.simplify(connection + 2 * x) == 0}


class SemilinearChart:
    """w = y / x^2 - 1 / kappa on one branch of x, with |D| = x^2 >= d_min and an inverse-chart
    transverse sensitivity |dw/dy| = 1 / x^2 <= cond_max. This is not a bound on
    the full chart Jacobian; physical-coordinate error transport is separate."""

    def __init__(self, kappa, branch_sign, d_min, cond_max):
        if (branch_sign not in (-1, 1)
                or not all(mp.isfinite(v) and v > 0 for v in (kappa, d_min, cond_max))):
            raise ValueError("invalid chart parameters")
        self.kappa, self.branch, self.d_min, self.cond_max = kappa, branch_sign, d_min, cond_max

    def _check(self, x):
        if not mp.isfinite(x) or not mp.isfinite(x * x):
            raise ChartDomainError("nonfinite coordinate or denominator")
        if x == 0:
            raise ChartDomainError("D = x^2 = 0: the chart is undefined")
        if (1 if x > 0 else -1) != self.branch:
            raise ChartDomainError("x left the chart's branch")
        if x * x < self.d_min:
            raise ChartDomainError(f"|D| = {x * x} below d_min = {self.d_min}")
        if 1 / (x * x) > self.cond_max:
            raise ChartDomainError(f"transverse inverse sensitivity {1 / (x * x)} above {self.cond_max}")

    def to_chart(self, x, y):
        self._check(x)
        if not mp.isfinite(y):
            raise ChartDomainError("nonfinite physical coordinate")
        value = y / (x * x) - 1 / self.kappa
        if not mp.isfinite(value):
            raise ChartDomainError("chart evaluation overflow")
        return value

    def from_chart(self, x, w):
        self._check(x)
        if not mp.isfinite(w):
            raise ChartDomainError("nonfinite chart coordinate")
        value = x * x * (w + 1 / self.kappa)
        if not mp.isfinite(value):
            raise ChartDomainError("chart reconstruction overflow")
        return value


def refuses(callable_):
    try:
        callable_()
    except ChartDomainError:
        return True
    return False


def fail_closed() -> dict:
    chart = SemilinearChart(kappa=40.0, branch_sign=1, d_min=1e-6, cond_max=1e5)
    cases = {
        "to_chart_D_zero": refuses(lambda: chart.to_chart(0.0, 1.0)),
        "to_chart_small_D": refuses(lambda: chart.to_chart(1e-4, 1.0)),
        "to_chart_ill_conditioned": refuses(
            lambda: SemilinearChart(40.0, 1, 1e-12, 1e3).to_chart(0.01, 1.0)),
        "to_chart_other_branch": refuses(lambda: chart.to_chart(-1.0, 1.0)),
        "from_chart_D_zero": refuses(lambda: chart.from_chart(0.0, 0.1)),
        "from_chart_small_D": refuses(lambda: chart.from_chart(1e-4, 0.1)),
        "from_chart_other_branch": refuses(lambda: chart.from_chart(-0.5, 0.1)),
        "valid_point_maps": not refuses(lambda: chart.from_chart(1.0, chart.to_chart(1.0, 0.2))),
    }
    return {"cases": cases, "holds": all(cases.values())}


def reference(kappa, eps, x0, y0, t_end):
    """The original ODE x' = x^2, y' = (-kappa + 2x) y + x^2 + eps x^3, by mpmath's Taylor solver."""
    f = mp.odefun(lambda t, z: [z[0] ** 2, (-kappa + 2 * z[0]) * z[1] + z[0] ** 2 + eps * z[0] ** 3],
                  0, [x0, y0])
    return f(t_end)


def chart_flow(kappa, eps, w0, t_end, keep_residual):
    """w' = -kappa w + (r_C - w r_D) / D with r_C = eps x^3, r_D = 0, D = x^2, x = 1/(1 - t)."""
    if not keep_residual:
        return w0 * mp.e ** (-kappa * t_end)
    g = mp.odefun(lambda t, w: -kappa * w + eps / (1 - t), 0, w0)
    return g(t_end)


def flows() -> dict:
    with mp.workdps(50):
        kappa, t_end, x0 = mp.mpf(40), mp.mpf(1) / 10, mp.mpf(1)
        w0 = mp.mpf(1) / 10
        y0 = x0**2 * (w0 + 1 / kappa)  # off the slow graph: a nonzero fast mode
        x_end = 1 / (1 - t_end)
        # 3. Exact model: the chart keeps the fast mode.
        xr, yr = reference(kappa, 0, x0, y0, t_end)
        y_chart = x_end**2 * (chart_flow(kappa, 0, w0, t_end, True) + 1 / kappa)
        fast = abs(y_chart - yr) / abs(yr)
        x_error = abs(xr - x_end) / x_end
        # 4. Perturbed model: keep the cofactor residual versus drop it.
        eps = mp.mpf(1) / 1000
        _, yp = reference(kappa, eps, x0, y0, t_end)
        y_kept = x_end**2 * (chart_flow(kappa, eps, w0, t_end, True) + 1 / kappa)
        y_dropped = x_end**2 * (chart_flow(kappa, eps, w0, t_end, False) + 1 / kappa)
        kept = abs(y_kept - yp) / abs(yp)
        dropped = abs(y_dropped - yp) / abs(yp)
        w_end = w0 * mp.e ** (-kappa * t_end)
        return {
            "fast_mode": {"kappa": 40, "w0": "1/10", "t": "1/10", "w_end": mp.nstr(w_end, 20),
                          "y_relative_difference": mp.nstr(fast, 5), "x_relative_difference": mp.nstr(x_error, 5),
                          "holds": bool(fast <= mp.mpf("1e-30") and x_error <= mp.mpf("1e-30") and w_end != 0)},
            "finite_epsilon": {"eps": "1/1000", "with_residual": mp.nstr(kept, 5),
                               "residual_dropped": mp.nstr(dropped, 5),
                               "holds": bool(kept <= mp.mpf("1e-30") and dropped > mp.mpf("1e-6"))},
        }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    identity = ratio_chart_identity()
    closed = fail_closed()
    flow = flows()
    connection = moving_frame_connection()
    gate = {
        "identity": identity["general_identity"] and identity["semilinear_exact"],
        "fail_closed": closed["holds"],
        "fast_mode_kept": flow["fast_mode"]["holds"],
        "finite_epsilon_not_replaced": flow["finite_epsilon"]["holds"],
        "connection_retained": bool(connection["equals_minus_2x"]) and identity["semilinear_a"] == "2*x",
    }
    report = {"schema": SCHEMA, "sympy": sp.__version__, "mpmath": mp.__version__, "identity": identity,
              "fail_closed": closed, "flows": flow, "moving_frame": connection, "gate": gate,
              "verdict": "PASS" if all(gate.values()) else "FAIL",
              "claim_ceiling": "model-specific chart theorem and tests; no production replacement"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"]}))
    return 0 if report["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
