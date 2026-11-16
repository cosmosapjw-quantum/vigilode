#!/usr/bin/env python3
"""Agreement of the Rust and the Python problem definitions (PREDECLARED.json,
'reference.model_agreement').

Reads the `SPECJ {"kind": "model_probe", ...}` lines of the Rust measurement
program (logs/measure_model_probes.log), evaluates the Python right-hand side and
Jacobian of the same model at the same time and state (decoded from the IEEE-754
bits) and prints one `MODELCHECK {json}` line per probe: the largest difference of
the right-hand sides and of the Jacobians, relative to the largest component.
A relative right-hand-side difference above 1e-9 is a blocker for that problem;
the exit status is 1 when any probe exceeds it.

Run:  python3 -I reference/check_models.py logs/measure_model_probes.log > logs/check_models.log 2>&1
"""
import json
import struct
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from models import adaptive_models, order_models  # noqa: E402

LIMIT = 1.0e-9


def number(bits):
    return struct.unpack(">d", bytes.fromhex(bits))[0]


def main(path):
    models = {
        "adaptive": {m["id"]: m for m in adaptive_models(0.27, 0.3)},
        "order": {m["id"]: m for m in order_models()},
    }
    worst, probes, complete = 0.0, 0, False
    for line in Path(path).read_text().splitlines():
        if not line.startswith("SPECJ "):
            continue
        record = json.loads(line[len("SPECJ "):])
        if record.get("kind") == "footer":
            complete = bool(record.get("complete"))
        if record.get("kind") != "model_probe":
            continue
        model = models[record["set"]][record["problem"]]
        t = number(record["t_bits"])
        y = np.array([number(b) for b in record["y_bits"]])
        f_rust = np.array([number(b) for b in record["f_bits"]])
        n = record["n"]
        j_rust = np.array([number(b) for b in record["jacobian_bits"]]).reshape(n, n)
        f_python = np.array(model["f"](t, y), dtype=float)
        j_python = np.array(model["jac"](t, y), dtype=float)
        f_scale = float(np.max(np.abs(f_python)))
        j_scale = float(np.max(np.abs(j_python)))
        f_defect = float(np.max(np.abs(f_rust - f_python)) / f_scale) if f_scale > 0.0 else 0.0
        j_defect = float(np.max(np.abs(j_rust - j_python)) / j_scale) if j_scale > 0.0 else 0.0
        worst = max(worst, f_defect)
        probes += 1
        print("MODELCHECK " + json.dumps({
            "set": record["set"], "problem": record["problem"], "seed": record["seed"], "n": n,
            "rhs_relative_difference": f_defect,
            "rhs_components_bit_identical": int(np.sum(f_rust == f_python)),
            "jacobian_relative_difference": j_defect,
            "within_limit": f_defect <= LIMIT,
        }, allow_nan=False))
    print("MODELCHECK " + json.dumps({
        "kind": "footer", "probes": probes, "probe_log_complete": complete,
        "largest_rhs_relative_difference": worst, "limit": LIMIT,
        "all_within_limit": worst <= LIMIT and probes > 0 and complete,
    }, allow_nan=False))
    return 0 if worst <= LIMIT and probes > 0 and complete else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
