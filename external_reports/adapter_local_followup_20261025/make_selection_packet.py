#!/usr/bin/env python3
"""Build the source-locked, same-checkpoint selection input from local raw.

This script only reads raw receipts and creates one new output.  Internal
solver timings are diagnostic; it never reclassifies the historical campaign.
"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def digest(data):
    if not isinstance(data, bytes):
        data = json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(data).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("run_root", type=Path)
    ap.add_argument("checkout", type=Path)
    ap.add_argument("output", type=Path)
    args = ap.parse_args()
    if args.output.exists():
        raise SystemExit("create-only output already exists")
    run = args.run_root / "narrow_measurement"
    index = json.loads((run / "INDEX.json").read_text())
    source = json.loads((args.run_root / "IDENTITY.json").read_text())
    tree = subprocess.check_output(
        ["git", "cat-file", "tree", source["source_tree"]], cwd=args.checkout
    )
    tree_sha = digest(tree)
    host_sha = digest({k: source[k] for k in ("os", "cpu_model", "affinity", "logical_cpus")})
    reference_crosscheck = json.loads(
        (args.run_root / "ANALYTIC_REFERENCE_CROSSCHECK.json").read_text()
    )
    analytical_uncertainty = {
        "chirp_grid": reference_crosscheck["results"]["chirp"]["reference_uncertainty_proposal"],
        "gyro_endpoint": reference_crosscheck["results"]["gyro"]["reference_uncertainty_proposal"],
        "rotation_switch": 1e-13,
        "stiff_switch": 1e-13,
    }
    descriptors = {
        "chirp_grid": ("exponential chirp with equivariant cubic residual", {"gamma": .8, "beta": 1.2, "omega0": 1., "epsilon": .3}, [1., -.8], 8., 1e-4, "129 full physical state samples"),
        "rotation_switch": ("rotation y'=100 Jy", {"omega": 100.}, [1., 0.], 1., 1e-5, "endpoint full 2D state"),
        "stiff_switch": ("y'=-10000(y-cos(t))-sin(t)", {"lambda": -10000.}, [1.], 1., 1e-5, "endpoint full scalar state"),
        "gyro_endpoint": ("catalog case10 uniform B relativistic gyromotion", {"omega_c": 500., "p0": [1., 0., .2]}, [0., 0., 0., 1., 0., .2], 1., 1e-4, "endpoint full 6D state"),
        "case5_endpoint": ("catalog case5 adiabatic flash", {"epsilon": 1e-6}, [0., 1.], 2., 1e-4, "endpoint full 2D state"),
    }
    case5_ref = args.checkout / "research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json"
    case5_data = json.loads(case5_ref.read_text())
    analytical_uncertainty["case5_endpoint"] = case5_data["uncertainty"]
    analytic_ref = {
        "chirp_grid": "high-precision analytic formula crosscheck in ANALYTIC_REFERENCE_CROSSCHECK.json",
        "gyro_endpoint": "high-precision analytic formula crosscheck in ANALYTIC_REFERENCE_CROSSCHECK.json",
        "rotation_switch": "cos(100), sin(100), binary64 versus mpmath80 difference zero at endpoint",
        "stiff_switch": "cos(1), binary64 versus mpmath80 difference zero at endpoint",
    }
    rows = []
    for receipt in index["rows"]:
        cohort = receipt["cohort"]
        if receipt["warmup"]:
            continue
        name, params, y0, tend, cap, workload = descriptors[cohort]
        block, candidate = receipt["block"], receipt["candidate"]
        stem = f"{cohort}_{block}_{candidate}"
        raw = run / f"{stem}.stdout"
        row_receipt = run / f"{stem}.receipt.json"
        if digest(raw.read_bytes()) != receipt["stdout_sha256"]:
            raise SystemExit(f"stdout hash mismatch: {raw}")
        if digest((run / f"{stem}.stderr").read_bytes()) != receipt["stderr_sha256"]:
            raise SystemExit(f"stderr hash mismatch: {stem}")
        if digest((run / f"{stem}.resource").read_bytes()) != receipt["resource_sha256"]:
            raise SystemExit(f"resource hash mismatch: {stem}")
        values = [json.loads(line) for line in raw.read_text().splitlines() if line.strip()]
        result = values[-1]
        if cohort == "chirp_grid":
            header = values[0]
            key = "chart" if candidate == "chart" else "physical"
            error = max(s[f"{key}_max_abs_error"] for s in values if s["kind"] == "sample")
            time = values[-1]["t"]
            timing = header["arm_timings"][key]
            cost = {"solve_seconds": timing["solve_seconds"], "setup_seconds": timing["setup_seconds"] + timing["reconstruction_seconds"], "probe_seconds": 0., "transition_seconds": 0., "total_seconds": timing["total_seconds"], "disjoint_components": True}
            success = header[f"{key}_success"] and header["time_grid_matches"]
        else:
            if cohort in ("rotation_switch", "stiff_switch"):
                error = result["endpoint_max_abs_error"]
                rec = result["record"]
                time = rec.get("final_time", rec.get("t"))
                if isinstance(time, list):
                    time = time[-1]
                elapsed = result["seconds"]
                success = result["success"] and result["accuracy_pass"]
            else:
                error = result["actual_error"]
                time = result["final_time"]
                elapsed = result["elapsed_seconds"]
                success = result["solver_success"] and result["status"] == "PASS_SCOPED"
            # These wrappers measure the whole solver invocation, including
            # setup, calibration, rejected pilots and transitions. Components
            # inside the envelope are not separately timed, so charge once.
            cost = {"solve_seconds": elapsed, "setup_seconds": 0., "probe_seconds": 0., "transition_seconds": 0., "total_seconds": elapsed, "disjoint_components": True}
        if time != tend:
            success = False
        if cohort == "case5_endpoint":
            reference_sha = digest(case5_ref.read_bytes())
            ref_kind = "empirical"
        else:
            reference_sha = digest({"description": analytic_ref[cohort], "uncertainty": analytical_uncertainty[cohort]})
            ref_kind = "analytic"
        scope = "per-arm setup+solve+reconstruction" if cohort == "chirp_grid" else "inclusive solver invocation; setup/probes/transition inside solve envelope"
        identity = {
            "problem_sha256": digest(name), "parameters_sha256": digest(params),
            "start_state_sha256": digest(y0), "t_start_hex": float(0).hex(), "t_end_hex": float(tend).hex(),
            "physical_metric_sha256": digest("max component absolute full original physical state"),
            "reference_sha256": reference_sha, "reference_kind": ref_kind,
            "error_ceiling_hex": float(cap).hex(), "host_sha256": host_sha,
            "source_tree_sha256": tree_sha, "build_config_sha256": digest({"profile": "measurement", "lock_sha256": source["cargo_lock_sha256"], "threads": 1}),
            "output_workload_sha256": digest(workload), "timing_scope_sha256": digest(scope), "environment_epoch": 0,
        }
        selected = candidate in ("chart", "selected_evidence", "selected_switch")
        rows.append({"window_id": cohort, "paired_block_id": str(block), "candidate_id": candidate,
                     "selected": selected, "identity": identity, "status": "completed" if success and receipt["exit_code"] == 0 else "solver_failed",
                     "final_time_hex": float(time).hex() if isinstance(time, (float, int)) else None,
                     "raw_verified": True, "receipt_sha256": digest(row_receipt.read_bytes()),
                     "binary_sha256": receipt["binary_sha256"], "reference_admitted": True,
                     "physical_error": error, "reference_uncertainty": analytical_uncertainty[cohort],
                     "selection_reason": "declared chart" if cohort == "chirp_grid" else "published adapter route",
                     "cost": cost})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as stream:
        json.dump({"schema_version": 1, "data_kind": "measured", "rows": rows}, stream, indent=2)
        stream.write("\n")
    print(f"wrote {len(rows)} rows to {args.output}")


if __name__ == "__main__":
    main()
