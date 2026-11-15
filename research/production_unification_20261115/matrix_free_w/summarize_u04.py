#!/usr/bin/env python3
"""Fold U04 raw logs (QUAL lines) into RESULTS.json.

Admissibility of a crossover row: status ok and endpoint WRMS difference to Dense <= 0.1 tolerance
units (both arms are the same method/controller, so this bounds the achieved-error mismatch).
A speedup of a failed or inadmissible row is not reported as a speedup.
"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def qual(path):
    out = {}
    for line in path.read_text().splitlines():
        if "QUAL {" in line:
            d = json.loads(line[line.index("QUAL ") + 5:])
            out[d["test"]] = d["data"]
    return out


def main():
    g = qual(HERE / "logs/u04_green.log")
    x = qual(HERE / "logs/u04_crossover.log")["c5_crossover"]
    rows = []
    for r in x:
        ok = r["status"] == "ok" and r["endpoint_wrms_diff_vs_dense_in_tol_units"] <= 0.1
        rows.append({k: r[k] for k in ("n", "arm", "threads", "status", "median_seconds", "repetitions",
                                        "warmups", "endpoint_wrms_diff_vs_dense_in_tol_units",
                                        "stored_f64_model")}
                    | {"admissible": ok,
                       "speedup_vs_dense": r["speedup_vs_dense"] if ok else None,
                       "accepted": r["steps"]["accepted"], "rejected": r["steps"]["rejected"],
                       "jvp_calls": r["steps"]["jvp_calls"], "lu": r["steps"]["lu"],
                       "linear_iterations": r["steps"]["linear_iterations"],
                       "inner_products": r["steps"]["inner_products"],
                       "startup": r["startup"]})
    best = {}
    for r in rows:
        if r["arm"] != "dense" and r["admissible"]:
            b = best.get(r["n"])
            if b is None or r["speedup_vs_dense"] > b["speedup_vs_dense"]:
                best[r["n"]] = {"arm": r["arm"], "speedup_vs_dense": r["speedup_vs_dense"]}
    crossover = [n for n, b in best.items() if b["speedup_vs_dense"] > 1.0]
    files = [HERE / "logs/u04_green.log", HERE / "logs/u04_crossover.log",
             ROOT / "crates/rodas5p-integrators/src/ppsw1.rs",
             ROOT / "crates/rodas5p-integrators/src/ppsw1_linear_backend.rs",
             ROOT / "crates/rodas5p-integrators/tests/ppsw1_linear_backend_contracts.rs"]
    res = {"schema": "vigilode.u04-results.v1", "task": "U04_COMMON_W_MATRIX_FREE",
           "scope": "Fisher MOL calibration, identity preconditioner, one host; diagnostic wall time "
                    "from a paired interleaved protocol (2 warmups, 7 repetitions); RSS NOT_RUN",
           "inputs_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in files},
           "parity": g["c1_dense_parity"], "failures": g["c2_failures"],
           "refusals": g["c3_refusals"], "budget": g["c4_budget"], "budget_guard": g["c6_budget_guard"],
           "crossover_rows": rows, "best_admissible_arm_per_n": best,
           "measured_crossover_n": sorted(crossover),
           "crossover_verdict": "NO_CROSSOVER_MEASURED" if not crossover else "CROSSOVER_AT_N"}
    (HERE / "RESULTS.json").write_text(json.dumps(res, indent=1, allow_nan=False) + "\n")
    print(res["crossover_verdict"], json.dumps(best))


if __name__ == "__main__":
    main()
