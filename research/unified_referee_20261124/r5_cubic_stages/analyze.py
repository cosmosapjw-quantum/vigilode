"""Compare native RODAS5P trajectories at the common output times with independent references."""
import argparse
import gzip
import json
from pathlib import Path

import numpy as np
import scipy
from scipy.integrate import solve_ivp
from taylor_reference import D, trajectory as taylor_path, discrepancy


def references(row):
    case = row["source"]
    n = case["n"]
    k = np.array(case["k"][:n])
    g = np.array(case["g"][:n])
    c = np.array(case["c"])[:n, :n]
    y0 = np.array(case["y0"][:n])
    times = np.array(row["times"])

    def rhs(_t, y):
        return -k*y - g*y**3 + c@y - np.sum(c, axis=1)*y

    refs = [
        solve_ivp(rhs, (times[0], times[-1]), y0, method=method, t_eval=times,
                  atol=5e-16, rtol=6e-14)
        for method in ("Radau", "DOP853")
    ]
    assert all(result.success and np.array_equal(times, result.t) for result in refs)
    paths = [result.y.T for result in refs]
    if case["name"] == "linear_exchange_control":
        # The eigenvectors of symmetric exchange are (1,1), (1,-1).
        mean = .5*(y0[0]+y0[1])*np.exp(-4*times)
        split = .5*(y0[0]-y0[1])*np.exp(-16*times)
        paths.insert(0, np.column_stack((mean+split, mean-split)))
    gap = max(float(np.max(np.abs(paths[0]-ref))) for ref in paths[1:])
    return paths[0], gap


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    opener = gzip.open if args.input.suffix == ".gz" else open
    with opener(args.input, "rt") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    results = {}
    for row in rows:
        name = row["source"]["name"]
        results.setdefault(name, []).append(row)

    records = []
    for name, group in results.items():
        scipy_ref, scipy_gap = references(group[0])
        source,grid = group[0]["source"], group[0]["times"]
        ref = taylor_path(source,grid,32,85)
        gap = float(discrepancy(ref,taylor_path(source,grid,22,60)))
        scipy_to_taylor = float(np.max(np.abs(np.asarray(ref,dtype=float)-scipy_ref)))
        grouped = {}
        for row in group:
            trajectory = np.asarray(row["trajectory"])
            assert np.array_equal(row["times"], group[0]["times"]) and np.shape(ref) == trajectory.shape
            achieved = float(discrepancy(ref, [[D(float(x)) for x in line] for line in trajectory]))
            grouped.setdefault(row["rtol"], {})[row["arm"]] = row, achieved
        for tol, arms in grouped.items():
            baseline, e = arms["Protected"]
            comparison = []
            for arm, (row, e_arm) in arms.items():
                work = row["client_work"]
                relative = abs(e_arm-e) / e if e else None
                ratio = None if e < 200*gap or relative is None else relative
                case_status = ("reference_resolution_limited" if ratio is None else
                               "within_1_percent" if ratio <= 0.01 else "different_achieved_error")
                delta = np.max(np.abs(np.array(row["trajectory"])-baseline["trajectory"]))
                comparison.append(dict(arm=arm, peak_abs_error=e_arm, error_relative_to_baseline=relative,
                                       same_error=case_status, max_abs_delta_to_protected=float(delta),
                                       attempts=row["attempts"], accepted=row["accepted"], rejected=row["rejected"],
                                       rhs=row["core_work"]["rhs_calls"], work=work))
            records.append(dict(source=name, rtol=tol, reference_crosscheck_max_abs=gap, scipy_pair_gap=scipy_gap, scipy_to_taylor=scipy_to_taylor, arms=comparison))
    assert len(records)==12 and all(len(r["arms"])==4 for r in records)
    print("sets", len(records), "SciPy", scipy.__version__)
    for r in records:
        print(r["source"], r["rtol"], [(x["arm"],x["same_error"],x["work"]["candidate_stages"],
            x["work"]["fallback_stages"],x["work"]["sweeps"]) for x in r["arms"]])
    summary = dict(schema="vigilode.r5.cubic-native-stages.v1", scipy=scipy.__version__, numpy=np.__version__,
                   reference_contract="Decimal Taylor orders 22/32, precisions 60/85; fine mesh at least twice as dense in the rate-limited regime. Analytic Decimal exponential on symmetric exchange. Radau/DOP853 cross-check rtol=6e-14, atol=5e-16. The reference gap estimates convergence empirically.",
                   parity_contract="Peak absolute infinity-norm difference over the common output grid. Relative difference of peak errors <=1%, conditioned on protected error >=200x reference-method disagreement.",
                   wall_measurement="Development-profile single-run diagnostics; not a speed benchmark.",records=records)
    args.output.write_text(json.dumps(summary, indent=2,allow_nan=False)+"\n")


if __name__ == "__main__":
    main()
