#!/usr/bin/env python3
"""Builds reference/reference.json from the raw reference logs (PREDECLARED.json,
section 'reference'). No number is typed in: every value is parsed from
logs/reference_scipy.log (`REFPY` lines) and logs/reference_julia.log (`REFJL`
lines), or computed here from a closed form with mpmath at 40 digits.

  y_ref = SciPy Radau, rtol 1e-13
  u_i   = |y_ref,i - y_A,i| + max(|y_ref,i - y_B,i|, |y_ref,i - y_C,i|)
          A = SciPy Radau rtol 1e-11, B = Julia RadauIIA5, C = Julia Rodas5P
  closed form: y_ref = closed form rounded to binary64, u_i = |y_ref,i| 2^-52

Run:  python3 -I reference/build_reference.py
"""
import hashlib
import json
import struct
import sys
from pathlib import Path

import mpmath as mp

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
from models import adaptive_models, order_models  # noqa: E402

COMBUSTION_T_END = 0.27
NLDIFF_T_END = 0.3


def number(bits):
    return struct.unpack(">d", bytes.fromhex(bits))[0]


def bits(x):
    return struct.pack(">d", float(x)).hex()


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def records(path, prefix):
    out = []
    for line in path.read_text().splitlines():
        if line.startswith(prefix + " "):
            out.append(json.loads(line[len(prefix) + 1:]))
    return out


def usable(run, t_end):
    """A run enters the rule only if it reports success and ended at t_end."""
    return (run is not None and run.get("success") is True and "y_bits" in run
            and number(run["t_last_bits"]) == t_end)


def differences(reference, other):
    absolute = [abs(a - b) for a, b in zip(reference, other)]
    relative = [d / abs(a) for d, a in zip(absolute, reference) if a != 0.0]
    return absolute, {"max_abs": max(absolute), "max_rel": max(relative) if relative else 0.0}


def closed_form(model, t_end):
    mp.mp.dps = 40
    exact = [float(v) for v in model["exact"](t_end)]
    return {
        "kind": "closed_form", "n": model["n"], "t_end": t_end,
        "y_ref_bits": [bits(v) for v in exact], "y_ref": exact,
        "u": [abs(v) * 2.0 ** -52 for v in exact],
        "uncertainty_defined": True,
        "rule": "closed form at 40 digits rounded to binary64; u_i = |y_ref,i| 2^-52",
    }


def main():
    scipy_log = ROOT / "logs" / "reference_scipy.log"
    julia_log = ROOT / "logs" / "reference_julia.log"
    scipy_records = records(scipy_log, "REFPY")
    julia_records = records(julia_log, "REFJL")
    complete = {
        "scipy": any(r.get("kind") == "footer" and r.get("complete") for r in scipy_records),
        "julia": any(r.get("kind") == "footer" and r.get("complete") for r in julia_records),
    }
    runs = {}
    for record in scipy_records + julia_records:
        if "run" in record:
            runs[(record["problem"], record["run"])] = record

    out = {
        "schema": "vigilode.loop-l1.v07-stage-a.reference.v1",
        "sources": {
            "scipy_log": {"path": "logs/reference_scipy.log", "sha256": sha256(scipy_log),
                          "complete": complete["scipy"]},
            "julia_log": {"path": "logs/reference_julia.log", "sha256": sha256(julia_log),
                          "complete": complete["julia"]},
        },
        "versions": {
            "scipy_header": next((r for r in scipy_records if r.get("kind") == "header"), None),
            "julia_header": next((r for r in julia_records if r.get("kind") == "header"), None),
            "mpmath": mp.__version__,
        },
        "jacobian_checks": {r["problem"]: r["largest_relative_column_defect"]
                            for r in scipy_records if r.get("kind") == "jacobian_check"},
        "adaptive": {},
        "order": {},
    }
    for model in adaptive_models(COMBUSTION_T_END, NLDIFF_T_END):
        ident, t_end = model["id"], model["t_span"][1]
        if "exact" in model:
            out["adaptive"][ident] = closed_form(model, t_end)
            continue
        entry = {"kind": "numerical", "n": model["n"], "t_end": t_end, "runs": {},
                 "differences": {}}
        for label in ("ref", "A", "B", "C"):
            run = runs.get((ident, label))
            if run is not None:
                entry["runs"][label] = {k: v for k, v in run.items() if k != "y_bits"}
                entry["runs"][label]["usable"] = usable(run, t_end)
        ref = runs.get((ident, "ref"))
        if not usable(ref, t_end):
            entry.update({"uncertainty_defined": False,
                          "obstacle": "the SciPy rtol = 1e-13 run is missing or failed"})
            out["adaptive"][ident] = entry
            continue
        y_ref = [number(b) for b in ref["y_bits"]]
        entry["y_ref_bits"] = ref["y_bits"]
        entry["y_ref"] = y_ref
        parts = {}
        for label in ("A", "B", "C"):
            run = runs.get((ident, label))
            if usable(run, t_end):
                parts[label], entry["differences"][label] = differences(
                    y_ref, [number(b) for b in run["y_bits"]])
        independent = [parts[label] for label in ("B", "C") if label in parts]
        if "A" not in parts or not independent:
            entry.update({"uncertainty_defined": False,
                          "obstacle": "run A is unusable" if "A" not in parts
                          else "both independent runs B and C are unusable"})
        else:
            entry["u"] = [a + max(values) for a, values in zip(parts["A"], zip(*independent))]
            entry["uncertainty_defined"] = True
            entry["independent_runs_used"] = [label for label in ("B", "C") if label in parts]
            if "C" in parts:
                # Exploratory only (not the predeclared rule): the uncertainty without the
                # least accurate independent run B.
                entry["u_without_b"] = [a + c for a, c in zip(parts["A"], parts["C"])]
        entry["rule"] = "u_i = |ref - A|_i + max(|ref - B|_i, |ref - C|_i)"
        out["adaptive"][ident] = entry
    for model in order_models():
        out["order"][model["id"]] = closed_form(model, model["t_span"][1])
    (HERE / "reference.json").write_text(json.dumps(out, indent=1, allow_nan=False) + "\n")
    for ident, entry in out["adaptive"].items():
        if entry["kind"] == "numerical":
            print("%-20s defined=%s  %s" % (ident, entry["uncertainty_defined"], json.dumps(
                {k: "%.2e" % v["max_rel"] for k, v in entry["differences"].items()})))
    return 0


if __name__ == "__main__":
    sys.exit(main())
