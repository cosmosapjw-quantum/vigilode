#!/usr/bin/env python3
"""Independently check stored cohort receipts; never execute a solver or timing run."""
from collections import defaultdict
from hashlib import sha256
import json
from pathlib import Path
from statistics import median
import tarfile


ROOT = Path(__file__).resolve().parents[1]
CAP = 1e-5


def read(relative):
    return json.loads((ROOT / relative).read_text())


def stored_json(folder):
    """Read individual evidence when present, or its archive without extracting it."""
    path = ROOT / "results" / folder
    archive = path / "RAW.tar.gz"
    if archive.exists():
        with tarfile.open(archive, "r:gz") as stream:
            for member in stream:
                if member.isfile() and member.name.endswith(".json"):
                    file = stream.extractfile(member)
                    assert file is not None
                    yield Path(member.name), json.load(file)
    else:
        for file in sorted(path.glob("*.json")):
            yield file, json.loads(file.read_text())


def main():
    causal = []
    causal_sources = set()
    causal_count = {}
    for folder in ("causal_campaign", "causal_campaign_threads2"):
        n = 0
        for path, obj in stored_json(folder):
            if "runs" not in obj or "source_commit" not in obj:
                continue
            causal_sources.add(obj["source_commit"])
            causal.extend(obj["runs"])
            n += len(obj["runs"])
            for row in obj["runs"]:
                assert len(row["observation_times"]) == 17
                assert len(row["observation_states"]) == 17
                assert row["observation_times"][0] == obj["time_interval"][0]
                assert row["observation_times"][-1] == obj["time_interval"][1]
        causal_count[folder] = n
    assert causal_count == {"causal_campaign": 126, "causal_campaign_threads2": 84}
    assert all(x["success"] and x["reaches_end"] for x in causal)
    causal_eligible = sum(x["max_abs_error"] <= CAP for x in causal)
    assert causal_eligible == 195

    switching = []
    switching_warmup = []
    for path, obj in stored_json("switching_campaign"):
        if "common_observation_max_abs_error" not in obj:
            continue
        (switching_warmup if path.stem.endswith("_-1") else switching).append(obj)
    assert (len(switching), len(switching_warmup)) == (120, 40)
    switching_success = sum(x["success"] for x in switching)
    assert switching_success == 114
    switching_eligible = sum(
        x["success"] and x["common_observations_covered"]
        and x["common_observation_max_abs_error"] <= CAP for x in switching
    )

    affine = read("results/AFFINE_CAMPAIGN.json")["rows"]
    assert len(affine) == 192
    assert sum(x["warmup"] for x in affine) == 48
    assert all(x["success"] and x["maximum_sampled_absolute_error"] <= CAP for x in affine)

    pde = [json.loads(line) for line in
           (ROOT / "results/PDE_NATIVE_BENCHMARK_FINAL.jsonl").read_text().splitlines()]
    groups = defaultdict(list)
    for row in pde:
        groups[(row["case"], row["method"])].append(row)
    assert len(pde) == 135 and len(groups) == 27
    assert all(len(group) == 5 for group in groups.values())
    assert all(x["relative_weighted_l2_error"] <= CAP and x["hot_allocations"] == 0 for x in pde)
    for group in read("results/PDE_NATIVE_SUMMARY.json")["groups"]:
        for method, stored in group["methods"].items():
            rows = groups[(group["case"], method)]
            assert abs(median(x["duration_ns"] / 1e9 for x in rows)
                       - stored["seconds_median"]) <= 1e-15
    summary = read("SUMMARY.json")
    assert summary["broad_production"] == "HOLD"
    assert summary["causal"]["completed"] == len(causal)
    assert summary["causal"]["same_error_cap_passes"] == causal_eligible
    assert summary["switching"]["completed"] == switching_success
    assert summary["pde"]["final_native_rows"] == len(pde)
    inputs = ["SUMMARY.json", "results/AFFINE_CAMPAIGN.json",
              "results/PDE_NATIVE_BENCHMARK_FINAL.jsonl", "results/PDE_NATIVE_SUMMARY.json",
              "results/CAUSAL_THREADS_AUDIT.json", "analysis/PRODUCTION_VALIDATION.md",
              "figures/frontier_performance.png", "figures/pde_scalability.png"]
    inputs.extend(str(path.relative_to(ROOT)) for path in sorted((ROOT / "results").glob("*/RAW.tar.gz")))
    result = {
        "schema": "vigilode.frontier-independent-evidence-audit.v1",
        "status": "PASS",
        "scope": "Read stored raw results and summary values; no solver execution, no new timing, no external qualification.",
        "causal": {"rows": len(causal), "counts": causal_count,
                   "successes": len(causal), "sampled_error_eligible": causal_eligible,
                   "all_rows_have_17_physical_observations_and_endpoints": True,
                   "source_commits": sorted(causal_sources)},
        "switching": {"timed_rows": len(switching), "warmups": len(switching_warmup),
                      "successes": switching_success, "sampled_error_eligible": switching_eligible},
        "affine": {"rows": len(affine), "timed_rows": 144, "warmups": 48,
                   "sampled_error_eligible": len(affine)},
        "pde": {"rows": len(pde), "groups": len(groups), "repeats": 5,
                "sampled_error_eligible": len(pde), "hot_allocation_max": 0,
                "max_error": max(x["relative_weighted_l2_error"] for x in pde),
                "max_relative_mass_drift": max(x["relative_mass_drift"] for x in pde),
                "summary_medians_match_raw": True},
        "limitations": ["Different cohorts have different methods, builds, and norms.",
                        "Passing a sampled error cap is not a continuous global error certificate.",
                        "Timing medians do not provide confidence intervals or cross-host speed guarantees.",
                        "Direct declared affine/harmonic actions are not general nonlinear adaptive solvers.",
                        "Native PDE cohort is a fixed linear prototype with calibrated controls and setup excluded."],
        "inputs_sha256": {name: sha256((ROOT / name).read_bytes()).hexdigest() for name in inputs},
    }
    out = ROOT / "results/FINAL_EVIDENCE_AUDIT.json"
    out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"status": result["status"], "causal_eligible": causal_eligible,
                      "switching_eligible": switching_eligible, "pde_rows": len(pde)}))


if __name__ == "__main__":
    main()
