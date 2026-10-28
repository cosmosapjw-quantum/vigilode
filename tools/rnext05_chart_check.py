#!/usr/bin/env python3
"""50-digit reference check of the chart provider runs (research node
research/rnext05_chart_provider_20261003, remaining-only DAG node R-NEXT-05).

The chart ODE x' = x^2, w' = -kappa w + eps x is autonomous, so each recorded point is
checked at the exact elapsed time t0 + sum of the binary64 steps (taken as exact reals):
x(T) = x0 / (1 - x0 T), w(T) = e^{-kappa tau} w(T_i) + eps int_{T_i}^{T_i + tau}
e^{-kappa (T_i + tau - s)} x(s) ds, w(0) = y0 / x0^2 - 1/kappa, y = x^2 (w + 1/kappa).
The model's eps (not the stepper's) is used, so the negative control runs with the forcing
omitted by the stepper but present in the model.
"""

from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path

import mpmath as mp

SCHEMA = "vigilode-rnext05-chart-provider-v1"


def unhex(text: str):
    return mp.mpf(struct.unpack(">d", bytes.fromhex(text))[0])


def check_run(run) -> dict:
    kappa, eps = unhex(run["kappa"]), unhex(run["eps_model"])
    x0, y0 = unhex(run["x0"]), unhex(run["y0"])
    x_of = lambda t: x0 / (1 - x0 * t)
    w = y0 / x0**2 - 1 / kappa

    def advance_w(w_start, t_start, tau):
        value = mp.e ** (-kappa * tau) * w_start
        if eps != 0 and tau > 0:
            value += eps * mp.quad(lambda s: mp.e ** (-kappa * (t_start + tau - s)) * x_of(s),
                                   [t_start, t_start + tau])
        return value

    def check(point, t, w_ref):
        x_ref = x_of(t)
        y_ref = x_ref**2 * (w_ref + 1 / kappa)
        y = unhex(point["y"])
        bound = unhex(point["b_phys"])
        boxes = (unhex(point["x_box"][0]) <= x_ref <= unhex(point["x_box"][1])
                 and unhex(point["w_box"][0]) <= w_ref <= unhex(point["w_box"][1]))
        return abs(y - y_ref) <= bound, boxes, abs(y - y_ref), bound, x_ref, y_ref

    t = mp.mpf(0)
    points = boxes_ok = physical_ok = 0
    worst_ratio = mp.mpf(0)
    violations = []
    outputs = []
    output_steps = set(run["output_steps"])
    for step in run["steps"]:
        for dense in step["dense"]:
            tau = unhex(dense["tau"])
            ok, box, err, bound, _, _ = check(dense, t + tau, advance_w(w, t, tau))
            points += 1
            physical_ok += ok
            boxes_ok += box
            if bound > 0:
                worst_ratio = max(worst_ratio, err / bound)
            if not ok and len(violations) < 5:
                violations.append({"t": mp.nstr(t + tau, 12), "error": mp.nstr(err, 6), "bound": mp.nstr(bound, 6)})
        tau = unhex(step["tau"])
        w_next = advance_w(w, t, tau)
        t = t + tau
        ok, box, err, bound, x_ref, y_ref = check(step, t, w_next)
        points += 1
        physical_ok += ok
        boxes_ok += box
        if bound > 0:
            worst_ratio = max(worst_ratio, err / bound)
        if not ok and len(violations) < 5:
            violations.append({"t": mp.nstr(t, 12), "error": mp.nstr(err, 6), "bound": mp.nstr(bound, 6)})
        if step["index"] in output_steps:
            slow = x_ref**2 / kappa
            outputs.append({"t": mp.nstr(t, 15), "y_ref": mp.nstr(y_ref, 12), "bound": mp.nstr(bound, 6),
                            "fast_amplitude": mp.nstr(abs(y_ref - slow), 6),
                            "fast_over_bound": mp.nstr(abs(y_ref - slow) / bound, 6) if bound > 0 else "inf",
                            "t_times_kappa": mp.nstr(t * kappa, 6)})
        w = w_next
    return {"label": run["label"], "points": points, "physical_enclosed": physical_ok, "boxes_enclosed": boxes_ok,
            "worst_error_over_bound": mp.nstr(worst_ratio, 6), "violations": violations, "outputs": outputs,
            "steps": len(run["steps"]), "rejected_attempts": run["rejected_attempts"], "refused": run["refused"],
            "controller_ok": run["controller_ok"], "tube_ok": run["tube_ok"]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output already exists: {args.output}")
    data = json.loads(args.runs.read_text())
    mp.mp.dps = 50
    rows = []
    for run in data["runs"]:
        row = check_run(run)
        rows.append(row)
        print(row["label"], row["points"], row["physical_enclosed"], row["boxes_enclosed"], row["worst_error_over_bound"], flush=True)
    negative = check_run(data["negative_control"])
    print("negative", negative["points"], negative["physical_enclosed"], flush=True)
    # Fast mode: the output nearest t = 1/kappa must resolve the fast amplitude by 10x.
    fast_ok = True
    for row in rows:
        near = [o for o in row["outputs"] if abs(mp.mpf(o["t_times_kappa"]) - 1) < mp.mpf("1e-6")]
        fast_ok &= bool(near) and all(mp.mpf(o["fast_over_bound"]) > 10 for o in near)
    native = data["native_gate"]
    gate = {
        "1_tube_and_enclosure": native["tube_regular"] and all(r["boxes_enclosed"] == r["points"] for r in rows),
        "2_physical_bound": all(r["physical_enclosed"] == r["points"] for r in rows) and native["all_main_runs_completed"],
        "3_fast_mode_retained": fast_ok,
        "4_controller": native["controller_uses_physical_bound"],
        "5_fail_closed": native["fail_closed"],
        "6_forcing_necessary": negative["physical_enclosed"] < negative["points"],
    }
    report = {"schema": SCHEMA, "mpmath": mp.__version__, "dps": mp.mp.dps, "runs": rows,
              "negative_control": negative, "refusals": {k: v for k, v in data["refusals"].items()
                                                          if not isinstance(v, dict)},
              "refusal_runs": {k: data["refusals"][k]["refused"] for k in ("blow_up", "denominator_margin")},
              "gate": gate, "verdict": "PASS" if all(gate.values()) else "FAIL"}
    args.output.write_text(json.dumps(report, indent=1) + "\n")
    print(json.dumps({"gate": gate, "verdict": report["verdict"]}))


if __name__ == "__main__":
    main()
