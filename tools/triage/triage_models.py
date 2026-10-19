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
        else:
            cells = int(match.group(1))
            f, jac = sbs.brusselator(cells)
            y0 = []
            for i in range(cells):
                x = (i + 1.0) / (cells + 1.0)
                y0 += [1.0 + math.sin(2.0 * math.pi * x), 3.0]
            source = f"tools/stiff_benchmark_scipy.py::brusselator({cells})"
        y0 = np.asarray(y0, dtype=np.float64)
        return Model(problem_id, f, jac, y0, len(y0), None, [], source)
    return None


def _reference_v2_models() -> dict[str, Model]:
    gr2 = reference_v2_tool()
    models = {}
    for problem in gr2.physical_problems():
        rhs, jac, initial, bandwidth = gr2.problem_runtime(problem)
        pid = problem["problem_id"]

        def segment(index, problem=problem, rhs=rhs):
            return gr2.branch_fixed_rhs(problem, index, rhs)

        models[pid] = Model(pid, rhs, jac, initial, problem["dimension"], bandwidth,
                            [float(b) for b in problem["mandatory_breakpoints"]],
                            f"tools/reference_v2/generate_references_v2.py::problem_runtime({problem['family']!r})",
                            segment)
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
