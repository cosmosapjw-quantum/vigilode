#!/usr/bin/env python3
"""NumPy models of component-triage problems, imported from the existing reference tools (never copied).

Sources (imported, not re-implemented):

* ``tools/stiff_benchmark_scipy.py``: right-hand side and dense analytic Jacobian of ``robertson``, ``hires``,
  ``van-der-pol-mu<MU>`` (``van_der_pol(mu)``) and ``brusselator-1d-<CELLS>`` (``brusselator(cells)``).
  That module takes its initial states from the Rust benchmark JSON, so the initial states of these four
  families are stated here, next to their Rust definitions (crates/rodas5p-integrators/src/problems.rs:
  ``robertson_problem`` [1, 0, 0], ``stiff_van_der_pol_problem`` [2, 0]; crates/rodas5p-cli/src/
  stiff_benchmark.rs: HIRES [1, 0, 0, 0, 0, 0, 0, 0.0057], Brusselator u_i = 1 + sin(2 pi x_i),
  v_i = 3, x_i = (i + 1) / (cells + 1)). ``tools/triage/triage_parity.py`` compares them bit for bit with the
  ``y0_hex`` the binary reports.
* ``tools/reference_v2/generate_references_v2.py``: ``physical_problems()`` / ``problem_runtime()`` /
  ``branch_fixed_rhs()`` of ScientificCorpusV2.1 (ids ``<family>-n<N>-v2``, ``...-grid<NX>x<NY>-v2.1`` and the
  four holdouts, with their mandatory breakpoints and segment right-hand sides).
* Written here from the Rust sources (no NumPy model existed): ``gray-scott-<N>`` (gray_scott.rs) and
  ``mass-affine`` (problems.rs::constant_affine_mass_problem, integrated as y' = M^{-1} f); see their
  functions for the citations.

A CORPUS problem may name its model with the optional key ``numpy_model`` (an id of this registry); otherwise
its own id is looked up. A problem without a model is reported by the callers as ``no_numpy_model``.
NumPy and SciPy are imported lazily; without them :func:`registry` raises :class:`ModelsUnavailable`.
"""

from __future__ import annotations

import importlib.util
import math
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parent
CLASSIC_PATTERNS = (
    (re.compile(r"^robertson$"), "robertson"),
    (re.compile(r"^hires$"), "hires"),
    (re.compile(r"^van-der-pol-mu(\d+(?:\.\d+)?)$"), "van-der-pol"),
    (re.compile(r"^brusselator-1d-(\d+)$"), "brusselator-1d"),
    (re.compile(r"^gray-scott-(\d+)$"), "gray-scott"),
    (re.compile(r"^mass-affine$"), "mass-affine"),
)


class ModelsUnavailable(RuntimeError):
    pass


@dataclass
class Model:
    id: str
    rhs: Callable[[float, Any], Any]
    jac: Callable[[float, Any], Any]
    y0: Any
    dimension: int
    bandwidth: int | None
    breakpoints: list[float]
    source: str
    segment_rhs: Callable[[int], Callable[[float, Any], Any]] = field(repr=False, default=None)
    # Parity view of the Rust problem: its f, Jacobian and constant mass matrix (M y' = f). They differ from
    # rhs/jac only for mass problems, whose rhs/jac are the transformed y' = M^{-1} f that SciPy integrates.
    parity_rhs: Callable[[float, Any], Any] | None = field(repr=False, default=None)
    parity_jac: Callable[[float, Any], Any] | None = field(repr=False, default=None)
    mass: Any = field(repr=False, default=None)
    # Optional model JVP (t, y, v) -> J v; without it J v is the model Jacobian times v.
    jvp: Callable[[float, Any, Any], Any] | None = field(repr=False, default=None)

    def rust_rhs(self, t: float, y):
        return (self.parity_rhs or self.rhs)(t, y)

    def rust_dense_jacobian(self, t: float, y):
        matrix = (self.parity_jac or self.jac)(t, y)
        return matrix.toarray() if hasattr(matrix, "toarray") else matrix

    def rust_jvp(self, t: float, y, v):
        """J v of the Rust problem's f: the model JVP if given, else (sparse or dense) Jacobian times v."""
        import numpy as np

        if self.jvp is not None:
            return np.asarray(self.jvp(t, y, v), dtype=np.float64)
        matrix = (self.parity_jac or self.jac)(t, y)
        return np.asarray(matrix @ np.asarray(v, dtype=np.float64), dtype=np.float64).reshape(-1)

    def rhs_for_segment(self, index: int):
        return self.segment_rhs(index) if self.segment_rhs is not None else self.rhs

    def dense_jacobian(self, t: float, y):
        matrix = self.jac(t, y)
        return matrix.toarray() if hasattr(matrix, "toarray") else matrix


_CACHE: dict[str, Any] = {}


def _load(name: str, path: Path):
    if name not in _CACHE:
        spec = importlib.util.spec_from_file_location(name, path)
        module = importlib.util.module_from_spec(spec)
        sys.modules[name] = module
        spec.loader.exec_module(module)
        _CACHE[name] = module
    return _CACHE[name]


def scipy_tool():
    """tools/stiff_benchmark_scipy.py (also the source of REFERENCE_RTOL and PARITY_LIMIT)."""
    try:
        return _load("stiff_benchmark_scipy", TOOLS / "stiff_benchmark_scipy.py")
    except ImportError as exc:
        raise ModelsUnavailable(f"numpy/scipy unavailable: {exc}") from None


def reference_v2_tool():
    ref_dir = TOOLS / "reference_v2"
    if str(ref_dir) not in sys.path:
        sys.path.insert(0, str(ref_dir))
    try:
        return _load("generate_references_v2", ref_dir / "generate_references_v2.py")
    except ImportError as exc:
        raise ModelsUnavailable(f"reference_v2 generator unavailable: {exc}") from None


def _classic(problem_id: str):
    import numpy as np

    sbs = scipy_tool()
    for pattern, family in CLASSIC_PATTERNS:
        match = pattern.match(problem_id)
        if not match:
            continue
        if family == "robertson":
            (f, jac), y0 = sbs.robertson(), [1.0, 0.0, 0.0]
            source = "tools/stiff_benchmark_scipy.py::robertson"
        elif family == "hires":
            (f, jac), y0 = sbs.hires(), [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057]
            source = "tools/stiff_benchmark_scipy.py::hires"
        elif family == "van-der-pol":
            (f, jac), y0 = sbs.van_der_pol(float(match.group(1))), [2.0, 0.0]
            source = f"tools/stiff_benchmark_scipy.py::van_der_pol({float(match.group(1))!r})"
        elif family == "gray-scott":
            return gray_scott(problem_id, int(match.group(1)))
        elif family == "mass-affine":
            return mass_affine(problem_id)
        else:
            cells = int(match.group(1))
            f, jac = sbs.brusselator(cells)
            y0 = []
            for i in range(cells):
                x = (i + 1.0) / (cells + 1.0)
                y0 += [1.0 + math.sin(2.0 * math.pi * x), 3.0]
            source = f"tools/stiff_benchmark_scipy.py::brusselator({cells})"
            y0 = np.asarray(y0, dtype=np.float64)
            return Model(problem_id, f, jac, y0, len(y0), None, [], source + "; JVP of stiff_benchmark.rs::"
                         "brusselator_jvp_only_problem (same equation, same operation order)",
                         jvp=brusselator_jvp(cells))
        y0 = np.asarray(y0, dtype=np.float64)
        return Model(problem_id, f, jac, y0, len(y0), None, [], source)
    return None


def brusselator_jvp(cells: int):
    """J w of the 1D Brusselator in the operation order of crates/rodas5p-cli/src/stiff_benchmark.rs::
    brusselator_jvp_only_problem (its right-hand side is brusselator_problem's: same c = (cells + 1)^2 / 50,
    boundary u = 1, v = 3, and y0). A J @ w product sums in another order and differs by rounding where the
    product cancels."""
    import numpy as np

    c = (cells + 1.0) ** 2 / 50.0

    def jvp(_t, y, w):
        y, w = np.asarray(y, dtype=np.float64), np.asarray(w, dtype=np.float64)
        u, v, du, dv = y[0::2], y[1::2], w[0::2], w[1::2]
        dul, dur = np.concatenate(([0.0], du[:-1])), np.concatenate((du[1:], [0.0]))
        dvl, dvr = np.concatenate(([0.0], dv[:-1])), np.concatenate((dv[1:], [0.0]))
        out = np.empty(2 * cells)
        out[0::2] = (2.0 * u * v - 4.0) * du + u * u * dv + c * (dul - 2.0 * du + dur)
        out[1::2] = (3.0 - 2.0 * u * v) * du - u * u * dv + c * (dvl - 2.0 * dv + dvr)
        return out

    return jvp


def gray_scott(problem_id: str, n_grid: int) -> Model:
    """Periodic 2D Gray-Scott on the unit square, n = 2 N^2, state [u (N^2, x fastest), v (N^2)].

    Source: crates/rodas5p-integrators/src/gray_scott.rs::gray_scott_2d_with_log_norm (D_u 2e-3, D_v 1e-3,
    F 0.04, k 0.06, dx = 1/N, 5-point periodic Laplacian (l + r + u + d - 4c) / dx^2, initial bump
    v = 0.25 exp(-100 |x - (0.5, 0.5)|^2), u = 1 - v at cell centres). The right-hand side and the JVP keep the
    Rust operation order, so parity is bit-level. SA-05 (coincident periodic neighbours counted twice for
    N = 2) does not arise: for N >= 3 the four neighbours of a cell are distinct; the Jacobian below sums
    duplicate entries anyway, which is the same convention.
    """
    import numpy as np
    from scipy.sparse import coo_matrix, csc_matrix, diags

    if n_grid < 2:
        raise ValueError("gray-scott requires N >= 2")
    n2 = n_grid * n_grid
    dx = 1.0 / float(n_grid)
    inv_dx2 = 1.0 / (dx * dx)
    d_u, d_v, f_feed, k_kill = 2.0e-3, 1.0e-3, 0.04, 0.06
    idx = np.arange(n2)
    i, j = idx % n_grid, idx // n_grid
    left = (i - 1) % n_grid + j * n_grid
    right = (i + 1) % n_grid + j * n_grid
    down = i + ((j - 1) % n_grid) * n_grid
    up = i + ((j + 1) % n_grid) * n_grid

    def lap(vals):
        return (vals[left] + vals[right] + vals[up] + vals[down] - 4.0 * vals) * inv_dx2

    def rhs(_t, y):
        y = np.asarray(y, dtype=np.float64)
        u, v = y[:n2], y[n2:]
        uv2 = u * v * v
        return np.concatenate((d_u * lap(u) - uv2 + f_feed * (1.0 - u),
                               d_v * lap(v) + uv2 - (f_feed + k_kill) * v))

    def jvp(_t, y, w):
        y, w = np.asarray(y, dtype=np.float64), np.asarray(w, dtype=np.float64)
        u, v, du, dv = y[:n2], y[n2:], w[:n2], w[n2:]
        return np.concatenate((d_u * lap(du) - v * v * du - 2.0 * u * v * dv - f_feed * du,
                               d_v * lap(dv) + v * v * du + 2.0 * u * v * dv - (f_feed + k_kill) * dv))

    rows = np.concatenate([idx] * 5)
    cols = np.concatenate([idx, left, right, down, up])
    vals = np.concatenate([np.full(n2, -4.0 * inv_dx2)] + [np.full(n2, inv_dx2)] * 4)
    laplacian = csc_matrix(coo_matrix((vals, (rows, cols)), shape=(n2, n2)))  # duplicates summed (SA-05)

    def jac(_t, y):
        from scipy.sparse import bmat

        y = np.asarray(y, dtype=np.float64)
        u, v = y[:n2], y[n2:]
        return csc_matrix(bmat([[d_u * laplacian + diags(-v * v - f_feed), diags(-2.0 * u * v)],
                                [diags(v * v), d_v * laplacian + diags(2.0 * u * v - (f_feed + k_kill))]]))

    y0 = np.zeros(2 * n2)
    for jj in range(n_grid):
        dy = (jj + 0.5) * dx - 0.5
        for ii in range(n_grid):
            dx_c = (ii + 0.5) * dx - 0.5
            bump = 0.25 * math.exp(-100.0 * (dx_c * dx_c + dy * dy))
            y0[ii + jj * n_grid] = 1.0 - bump
            y0[n2 + ii + jj * n_grid] = bump
    return Model(problem_id, rhs, jac, y0, 2 * n2, None, [],
                 f"tools/triage/triage_models.py::gray_scott({n_grid}) from crates/rodas5p-integrators/src/"
                 f"gray_scott.rs", jvp=jvp)


def mass_affine(problem_id: str) -> Model:
    """Constant noncommuting mass, affine right-hand side: M y' = J y + r(t), r = r0 + t r1 + t^2 r2.

    Source: crates/rodas5p-integrators/src/problems.rs::constant_affine_mass_problem (M = [[2, 1], [0, 3]],
    J = [[-4, 1], [2, -5]], r0 = [1/2, -2/3], r1 = [3/7, 5/11], r2 = [-1/13, 2/17], y0 = [2/5, -1/3]).
    SciPy solve_ivp has no mass matrix, so the model integrates y' = M^{-1} f with an exact direct solve of
    the constant M (numpy.linalg.solve at every evaluation) and Jacobian M^{-1} J; parity compares the Rust
    f, J and M themselves (`parity_rhs`, `parity_jac`, `mass`).
    """
    import numpy as np

    m = np.array([[2.0, 1.0], [0.0, 3.0]])
    jm = np.array([[-4.0, 1.0], [2.0, -5.0]])
    r0, r1, r2 = (0.5, -2.0 / 3.0), (3.0 / 7.0, 5.0 / 11.0), (-1.0 / 13.0, 2.0 / 17.0)

    def f(t, y):
        y = np.asarray(y, dtype=np.float64)
        return np.array([jm[0, 0] * y[0] + jm[0, 1] * y[1] + (r0[0] + t * r1[0] + t * t * r2[0]),
                         jm[1, 0] * y[0] + jm[1, 1] * y[1] + (r0[1] + t * r1[1] + t * t * r2[1])])

    minv_j = np.linalg.solve(m, jm)
    return Model(problem_id, lambda t, y: np.linalg.solve(m, f(t, y)), lambda t, y: minv_j,
                 np.array([2.0 / 5.0, -1.0 / 3.0]), 2, None, [],
                 "tools/triage/triage_models.py::mass_affine from crates/rodas5p-integrators/src/problems.rs::"
                 "constant_affine_mass_problem; integrated as y' = M^{-1} f (numpy.linalg.solve with the constant "
                 "M), parity on f, J and M",
                 parity_rhs=f, parity_jac=lambda t, y: jm, mass=m)


def semilinear_jvp(legacy, dimension: int):
    """J v of the v2.1 semilinear advection-diffusion problem, built from tools/reference_v2/
    generate_references.py (semilinear_operator, semilinear_ramp) in the Rust JVP's order:
    out = A(t) v, then out_i += 2 nonlinear(t) y_i v_i."""
    import numpy as np

    nx, ny = legacy.semilinear_grid_shape(dimension)

    def jvp(t, y, v):
        y, v = np.asarray(y, dtype=np.float64), np.asarray(v, dtype=np.float64)
        ramp, _ = legacy.semilinear_ramp(t)
        nonlinear = 2.0 + 48.0 * ramp
        return legacy.semilinear_operator(nx, ny, t, v) + 2.0 * nonlinear * y * v

    return jvp


def _reference_v2_models() -> dict[str, Model]:
    gr2 = reference_v2_tool()
    models = {}
    for problem in gr2.physical_problems():
        rhs, jac, initial, bandwidth = gr2.problem_runtime(problem)
        pid = problem["problem_id"]

        def segment(index, problem=problem, rhs=rhs):
            return gr2.branch_fixed_rhs(problem, index, rhs)

        jvp = None
        source = f"tools/reference_v2/generate_references_v2.py::problem_runtime({problem['family']!r})"
        if problem["family"] == "semilinear-advection-diffusion-ramped":
            jvp = semilinear_jvp(gr2.legacy, problem["dimension"])
            source += ("; JVP = legacy.semilinear_operator(v) + 2 nonlinear y v, the operation order of "
                       "crates/rodas5p-integrators/src/scientific_corpus_v2.rs::semilinear_advection_diffusion_v2")
        models[pid] = Model(pid, rhs, jac, initial, problem["dimension"], bandwidth,
                            [float(b) for b in problem["mandatory_breakpoints"]], source, segment, jvp=jvp)
    return models


def model_for(problem: dict) -> Model | None:
    """The NumPy model of a CORPUS problem (optional alias key `numpy_model`), or None."""
    key = problem.get("numpy_model") or problem["id"]
    classic = _classic(key)
    if classic is not None:
        return classic
    if "v2" not in _CACHE:
        _CACHE["v2"] = _reference_v2_models()
    return _CACHE["v2"].get(key)


def solve_on_grid(model: Model, t_span: list[float], times: list[float], method: str, rtol: float, atol: float,
                  first_step: float | None = None, max_step: float | None = None) -> tuple[Any, dict]:
    """SciPy solve_ivp on the requested output times, restarted at the model's breakpoints.

    Breakpoints are integration-segment boundaries, not outputs (contract section 1); segment i uses the
    model's segment right-hand side. Radau/BDF get the analytic Jacobian as the model gives it (dense or sparse);
    LSODA gets it banded when the model declares a bandwidth, dense otherwise. Returns (states with one row per
    requested time, or None on failure; counters {nfev, njev, nlu}, success, message).
    """
    import numpy as np
    from scipy.integrate import solve_ivp

    t0, tf = float(t_span[0]), float(t_span[1])
    bounds = [t0] + [b for b in model.breakpoints if t0 < b < tf] + [tf]
    state = np.asarray(model.y0, dtype=np.float64)
    out: dict[int, Any] = {}
    info = {"method": method, "rtol": rtol, "atol": atol, "first_step": first_step, "max_step": max_step,
            "nfev": 0, "njev": 0, "nlu": 0, "success": True, "message": None, "segments": len(bounds) - 1}
    for seg, (a, b) in enumerate(zip(bounds, bounds[1:])):
        wanted = [i for i, t in enumerate(times) if (a < t <= b) or (seg == 0 and t == a)]
        t_eval = sorted({times[i] for i in wanted} | {b})
        options: dict[str, Any] = {"method": method, "rtol": rtol, "atol": atol,
                                   "t_eval": np.asarray(t_eval, dtype=np.float64)}
        if first_step is not None:
            options["first_step"] = min(first_step, b - a)
        if max_step is not None:
            options["max_step"] = max_step
        if method in ("Radau", "BDF"):
            options["jac"] = model.jac
        elif method == "LSODA":
            if model.bandwidth is None:
                options["jac"] = model.dense_jacobian
            else:
                gr2 = reference_v2_tool()
                bw = model.bandwidth
                options.update(lband=bw, uband=bw, jac=lambda t, y, bw=bw: gr2.packed_banded(model.jac(t, y), bw, bw))
        else:
            raise ValueError(f"unsupported SciPy method {method!r}")
        try:
            result = solve_ivp(model.rhs_for_segment(seg), (a, b), state, **options)
        except Exception as exc:  # SciPy raises plain ValueError/LinAlgError on hard failures
            info.update(success=False, message=f"{type(exc).__name__}: {exc}")
            return None, info
        info["nfev"] += int(result.nfev)
        info["njev"] += int(result.njev)
        info["nlu"] += int(result.nlu)
        if not result.success or result.y.shape[1] != len(t_eval):
            info.update(success=False, message=str(result.message))
            return None, info
        columns = {float(t): result.y[:, k].copy() for k, t in enumerate(t_eval)}
        for i in wanted:
            out[i] = columns[float(times[i])]
        state = result.y[:, -1].copy()
    if len(out) != len(times):
        info.update(success=False, message=f"{len(times) - len(out)} requested times outside the span")
        return None, info
    states = np.asarray([out[i] for i in range(len(times))], dtype=np.float64)
    return states, info


def versions() -> dict:
    import platform

    import numpy as np
    import scipy

    return {"python": platform.python_version(), "numpy": np.__version__, "scipy": scipy.__version__,
            "machine": platform.machine()}


def require_numpy() -> None:
    try:
        import numpy  # noqa: F401
        import scipy  # noqa: F401
    except ImportError as exc:
        raise ModelsUnavailable(f"numpy/scipy unavailable: {exc}") from None
