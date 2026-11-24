#!/usr/bin/env python3
"""References of V27 phase 2 (node V27_ROUTER_HYPERPARAMETER_FREE, PREDECLARED.json measures).

Closed form where the pre-registration allows it (linear problems, oscillator, heat by its modes,
the scalar finite-difference problem by its derived closed form), evaluated with mpmath at 50
digits and rounded to the nearest double; otherwise Rodas5p::solve at rtol 1e-12, atol 1e-14
through the evaluation runner (arm R of the N binary, entry log off), with the default h0 and
max_steps, and once more with --max-steps 10000000 if that run does not succeed. Every runner
command, exit code and raw output line is kept in the output.

Usage: python3 -I references.py            writes logs/phase2/references.json
"""
import json
import os
import subprocess
import sys
import time

import mpmath

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "logs", "phase2", "references.json")
RUNNER_N = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v27/release/v27-runner"
TASKSET = ["taskset", "-c", "0-5,12-17"]
mpmath.mp.dps = 50


def to_double(x):
    return float(mpmath.mpf(x))


def bits(x):
    import struct

    return struct.pack(">d", x).hex()


def heat(n, y0, t):
    """y' = K y, K = tridiag(1, -2, 1) (n+1)^2, by the modes of K."""
    m = n + 1
    v = [[mpmath.sin(k * mpmath.pi * i / m) for i in range(1, n + 1)] for k in range(1, n + 1)]
    lam = [-4 * m**2 * mpmath.sin(k * mpmath.pi / (2 * m)) ** 2 for k in range(1, n + 1)]
    c = [mpmath.mpf(2) / m * mpmath.fsum(y0[i] * v[k][i] for i in range(n)) for k in range(n)]
    return [mpmath.fsum(c[k] * mpmath.exp(lam[k] * t) * v[k][i] for k in range(n)) for i in range(n)]


def heat_sine(n):
    return [mpmath.sin(mpmath.pi * i / (n + 1)) for i in range(1, n + 1)]


def scalar(t):
    k = mpmath.mpf(10) ** 6
    return [(k**2 * mpmath.cos(t) + k * mpmath.sin(t)) / (1 + k**2) + mpmath.exp(-k * t) / (1 + k**2)]


def closed_forms():
    one = mpmath.mpf(1)
    a = mpmath.mpf("0.1")
    w = mpmath.sqrt(mpmath.mpf(10) ** 4 - a**2)
    t10 = mpmath.mpf(10)
    forms = {
        "lin_2000": ("exp(-2000 t), t = 1", [mpmath.exp(-2000 * one)]),
        "lin_3200": ("exp(-3200 t), t = 1", [mpmath.exp(-3200 * one)]),
        "lin_3400": ("exp(-3400 t), t = 1", [mpmath.exp(-3400 * one)]),
        "osc_1": ("(cos t, -sin t), t = 1", [mpmath.cos(1), -mpmath.sin(1)]),
        "osc_10": ("(cos t, -sin t), t = 10", [mpmath.cos(10), -mpmath.sin(10)]),
        "osc_1000": ("(cos t, -sin t), t = 1000", [mpmath.cos(1000), -mpmath.sin(1000)]),
        "damped_osc": (
            "y1 = e^{-a t}(cos w t + (a/w) sin w t), y2 = -e^{-a t} (1e4/w) sin w t, "
            "a = 0.1, w = sqrt(1e4 - a^2), t = 10",
            [
                mpmath.exp(-a * t10) * (mpmath.cos(w * t10) + a / w * mpmath.sin(w * t10)),
                -mpmath.exp(-a * t10) * (mpmath.mpf(10) ** 4 / w) * mpmath.sin(w * t10),
            ],
        ),
        "nonnormal": (
            "y1 = 1001 e^{-t} - 1000 e^{-2t}, y2 = e^{-2t}, t = 10",
            [1001 * mpmath.exp(-t10) - 1000 * mpmath.exp(-2 * t10), mpmath.exp(-2 * t10)],
        ),
        "F2": ("e^{-t}, t = 1", [mpmath.exp(-one)]),
        "N2": ("(1, e^{-1000 t}), t = 1", [one, mpmath.exp(-1000 * one)]),
        "P_scalar_fd": (
            "(k^2 cos t + k sin t)/(1 + k^2) + e^{-k t}/(1 + k^2), k = 1e6, t = 1",
            scalar(one),
        ),
        "P_scalar_dense": (
            "(k^2 cos t + k sin t)/(1 + k^2) + e^{-k t}/(1 + k^2), k = 1e6, t = 1",
            scalar(one),
        ),
        "P_heat_fd": ("heat modes, n = 8, y0 = sin(pi x), t = 100", heat(8, heat_sine(8), 100)),
        "heat_64_fd": ("heat modes, n = 64, y0 = sin(pi x), t = 10", heat(64, heat_sine(64), 10)),
        "P_heat_dense": ("heat modes, n = 8, y0 = sin(pi x), t = 10", heat(8, heat_sine(8), 10)),
        "P_heat_jvp": (
            "heat modes, n = 8, y0 = 1, t = 0.01",
            heat(8, [one] * 8, mpmath.mpf("0.01")),
        ),
        "P_heat_banded": (
            "heat modes, n = 8, y0 = 1, t = 0.01",
            heat(8, [one] * 8, mpmath.mpf("0.01")),
        ),
    }
    return forms


NUMERICAL = [
    "robertson_40", "robertson_1e5", "hires", "orego", "vdp_1000", "vdp_1", "flame_1e-2",
    "flame_1e-3", "flame_1e-4", "kt_vanishing", "lotka_volterra", "lorenz", "arenstorf",
    "brusselator_2", "lorenz96_40", "pleiades", "P_mass",
]
NONE = {
    "P_fatal": "fatal for t > 0.3: no arm reaches t1",
    "P_blowup": "y' = y^2, y(0) = 1 blows up at t = 1 < t1 = 2",
}


def numerical(problem):
    attempts = []
    for extra in ([], ["--max-steps", "10000000"]):
        command = TASKSET + [RUNNER_N, "--arm", "R", "--problem", problem, "--rtol", "1e-12",
                             "--atol", "1e-14", "--no-log"] + extra
        done = subprocess.run(command, capture_output=True, text=True)
        row = {"command": " ".join(command), "exit_code": done.returncode,
               "stdout": done.stdout.strip(), "stderr": done.stderr.strip()}
        attempts.append(row)
        if done.returncode == 0:
            line = json.loads(done.stdout)
            if line["run"].get("success"):
                return line, attempts
    return None, attempts


def main():
    started = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    refs = {}
    for problem, (formula, values) in closed_forms().items():
        y = [to_double(v) for v in values]
        refs[problem] = {"kind": "closed-form", "formula": formula,
                         "y": y, "y_bits": [bits(v) for v in y],
                         "y_50_digits": [mpmath.nstr(v, 50) for v in values]}
    for problem in NUMERICAL:
        line, attempts = numerical(problem)
        entry = {"kind": "rodas5p", "rtol": 1e-12, "atol": 1e-14, "runs": attempts}
        if line is None:
            entry["y"] = None
            entry["note"] = "no successful reference run"
        else:
            run = line["run"]
            entry["t1_bits"] = run["final_time_bits"]
            entry["y"] = run["final_state"]
            entry["y_bits"] = run["final_state_bits"]
            entry["attempts"] = run["attempts"]
        refs[problem] = entry
    for problem, why in NONE.items():
        refs[problem] = {"kind": "none", "why": why, "y": None}
    out = {"schema": "vigilode.v27.phase2.references.v1", "started_utc": started,
           "finished_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
           "runner": RUNNER_N, "mpmath_dps": mpmath.mp.dps, "references": refs}
    text = json.dumps(out, indent=1, sort_keys=True, allow_nan=False)
    with open(OUT, "w") as handle:
        handle.write(text + "\n")
    bad = [p for p, r in refs.items() if r["kind"] == "rodas5p" and r["y"] is None]
    print(json.dumps({"references": len(refs), "without_successful_run": bad}))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
