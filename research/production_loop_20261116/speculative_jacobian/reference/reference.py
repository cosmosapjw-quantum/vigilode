#!/usr/bin/env python3
"""SciPy references of loop node V07 (PREDECLARED.json, section 'reference').

For every numerical problem of the adaptive set: SciPy Radau with the analytic
dense Jacobian at rtol = 1e-13 (the reference value) and at rtol = 1e-11 (run A
of the uncertainty). Prints one `REFPY {json}` line per run; states are printed
as the hexadecimal IEEE-754 bits of each component. Nothing is written to disk:
redirect the output to logs/reference_scipy.log.

Run:  python3 -I reference/reference.py > logs/reference_scipy.log 2>&1
"""
import json
import struct
import sys
import time
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp

# `python3 -I` does not put the script directory on the path.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from models import adaptive_models  # noqa: E402

COMBUSTION_T_END = 0.27
NLDIFF_T_END = 0.3
RTOL_MIN_MEASURED = 1.0e-8


def bits(x):
    return struct.pack(">d", float(x)).hex()


def ref_atol(model):
    """1e-6 of the measurement atol at rtol = 1e-8; e5: 1.7e-30."""
    if "atol_fixed" in model:
        return model["atol_fixed"] * 1.0e-6
    return RTOL_MIN_MEASURED * model["atol_scale"] * 1.0e-6


def jacobian_defect(model):
    """Largest entry defect of the analytic Jacobian against central differences at a
    state near y0, relative to the largest entry of its column (a sanity check of the
    Jacobian handed to Radau)."""
    y = np.array(model["y0"], dtype=float)
    scale = np.max(np.abs(y))
    y = y * (1.0 + 0.05 * np.sin(np.arange(1, len(y) + 1))) + 1.0e-3 * scale * (
        0.5 + 0.25 * np.cos(np.arange(1, len(y) + 1)))
    t = model["t_span"][0] + 0.1
    analytic = np.array(model["jac"](t, y), dtype=float)
    worst = 0.0
    for j in range(len(y)):
        delta = 1.0e-4 * (abs(y[j]) + 1.0e-3 * scale)
        plus, minus = y.copy(), y.copy()
        plus[j] += delta
        minus[j] -= delta
        column = (np.array(model["f"](t, plus)) - np.array(model["f"](t, minus))) / (2.0 * delta)
        reference = np.max(np.abs(analytic[:, j]))
        if reference > 0.0:
            worst = max(worst, float(np.max(np.abs(column - analytic[:, j])) / reference))
    return worst


def run(model, rtol, atol, label):
    start = time.time()
    record = {"problem": model["id"], "n": model["n"], "run": label, "method": "Radau",
              "rtol": rtol, "atol": atol, "t_end": model["t_span"][1]}
    try:
        sol = solve_ivp(model["f"], model["t_span"], model["y0"], method="Radau",
                        jac=model["jac"], rtol=rtol, atol=atol)
        record.update({
            "success": bool(sol.success), "status": int(sol.status), "message": str(sol.message),
            "t_last_bits": bits(sol.t[-1]), "y_bits": [bits(v) for v in sol.y[:, -1]],
            "steps": int(len(sol.t) - 1), "nfev": int(sol.nfev), "njev": int(sol.njev),
            "nlu": int(sol.nlu),
        })
    except Exception as error:  # a failed reference run is recorded, not hidden
        record.update({"success": False, "message": "exception: %r" % (error,)})
    record["wall_seconds_diagnostic_only"] = round(time.time() - start, 3)
    print("REFPY " + json.dumps(record, allow_nan=False), flush=True)


def main():
    print("REFPY " + json.dumps({"kind": "header", "python": sys.version.split()[0],
                                 "scipy": scipy.__version__, "numpy": np.__version__,
                                 "combustion_t_end": COMBUSTION_T_END,
                                 "nldiff_t_end": NLDIFF_T_END}), flush=True)
    models = adaptive_models(COMBUSTION_T_END, NLDIFF_T_END)
    count = 0
    for model in models:
        if "exact" in model:
            print("REFPY " + json.dumps({"kind": "closed_form", "problem": model["id"],
                                         "note": "reference is the closed form"}), flush=True)
            continue
        atol = ref_atol(model)
        print("REFPY " + json.dumps({"kind": "jacobian_check", "problem": model["id"],
                                     "largest_relative_column_defect": jacobian_defect(model)},
                                    allow_nan=False), flush=True)
        run(model, 1.0e-13, atol, "ref")
        run(model, 1.0e-11, 100.0 * atol, "A")
        count += 2
    print("REFPY " + json.dumps({"kind": "footer", "runs": count, "complete": True}), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
