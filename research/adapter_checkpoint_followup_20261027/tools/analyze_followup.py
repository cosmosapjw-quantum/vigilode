#!/usr/bin/env python3
"""Gates G1-G5 and diagnostic tables of the follow-up node (PREREGISTRATION.md)."""
import argparse
import importlib.util
import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location(
    "packet_pairs", REPO / "research/adapter_integration_20261027/benchmark/analyze_pairs.py")
packet = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packet)
PACKET_RAW = REPO / "research/adapter_integration_20261027/benchmark/raw_r2"


def norm(record):
    return json.dumps(record, sort_keys=True).replace("krylov-ws-legacy-v1", "krylov-ws-ls-workspace-v1")


def tests(receipt_dir):
    s = (receipt_dir / "stdout.log").read_text()
    r = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", s)
    receipt = json.loads((receipt_dir / "RECEIPT.json").read_text())
    return {"suites": len(r), "passed": sum(int(x[1]) for x in r), "failed": sum(int(x[2]) for x in r),
            "ignored": sum(int(x[3]) for x in r), "exit_code": receipt["exit_code"],
            "source_unchanged": receipt["source_unchanged_during_command"]}


def total_work(record):
    return sum(s["work_units"] for s in record["stats"]["segments"])


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("run", type=Path, help="registered run directory")
    p.add_argument("output", type=Path)
    a = p.parse_args()
    R = a.run
    res = {"schema": "vigilode.adapter-followup-results.v1", "timing_authority": "diagnostic_only"}
    # G1: identity of all packet records under the candidate binary.
    diffs, compared = [], 0
    for f in sorted((R / "identity").glob("*.stdout.json")):
        new = json.loads(f.read_text())["initial_records"]
        old = json.loads((PACKET_RAW / f.name).read_text())["initial_records"]
        for arm in old:
            compared += 1
            if norm(old[arm]) != norm(new[arm]):
                diffs.append(f"{f.name}:{arm}")
    res["G1_identity"] = {"pairs": len(list((R / "identity").glob("*.stdout.json"))),
                          "records_compared": compared, "differences": diffs,
                          "pass": compared == 52 and not diffs}
    # G2: facade matrix-free allocations per attempt.
    alloc = json.loads((R / "ALLOC.json").read_text())
    res["G2_allocations"] = {"rows": alloc["rows"],
                             "pass": all(r["candidate_over_base_marginal"] <= 0.10 for r in alloc["rows"])}
    # G3: stale-return sentinel versus adaptive, same binary.
    sr = json.loads((R / "pairs/stale-return__adaptive__adaptive-sentinel.stdout.json").read_text())
    ad, se = sr["initial_records"]["adaptive"], sr["initial_records"]["adaptive-sentinel"]
    res["G3_sentinel_stale_return"] = {
        "adaptive": {"work_units": total_work(ad), "rhs": ad["rhs"], "attempts": ad["attempts"],
                     "error": ad["max_component_absolute_error"], "accuracy_pass": ad["accuracy_pass"]},
        "sentinel": {"work_units": total_work(se), "rhs": se["rhs"], "attempts": se["attempts"],
                     "error": se["max_component_absolute_error"], "accuracy_pass": se["accuracy_pass"]},
        "pass": total_work(se) < total_work(ad) and ad["accuracy_pass"] and se["accuracy_pass"]}
    # G4: sentinel accuracy on every registered recheck cell.
    g4 = []
    for case in ["case5", "stiff-forced", "relaxation", "transition-39"]:
        d = json.loads((R / f"pairs/{case}__adaptive__adaptive-sentinel.stdout.json").read_text())
        x, y = d["initial_records"]["adaptive"], d["initial_records"]["adaptive-sentinel"]
        g4.append({"case": case, "adaptive_work": total_work(x), "sentinel_work": total_work(y),
                   "adaptive_rhs": x["rhs"], "sentinel_rhs": y["rhs"],
                   "sentinel_over_adaptive_work": total_work(y) / total_work(x),
                   "sentinel_accuracy_pass": y["accuracy_pass"],
                   "sentinel_error": y["max_component_absolute_error"]})
    res["G4_sentinel_accuracy"] = {"rows": g4, "pass": all(r["sentinel_accuracy_pass"] for r in g4)}
    # G5: tests.
    res["G5_tests"] = {name: tests(R / name) for name in ["tests_vigilode", "tests_integrators", "tests_krylov"]}
    res["G5_tests"]["pass"] = all(v["failed"] == 0 and v["exit_code"] == 0 and v["source_unchanged"]
                                  for k, v in res["G5_tests"].items() if k != "pass")
    # Diagnostic pairs (packet qualification rules, unchanged).
    rows = []
    for f in sorted((R / "pairs").glob("*.stdout.json")):
        rows.append(packet.analyze(json.loads(f.read_text())))
    res["pairs"] = rows
    res["cross_binary"] = json.loads((R / "cross_binary/CROSS_BINARY.json").read_text())
    res["verdict"] = "PASS" if all(res[g]["pass"] for g in
                                   ["G1_identity", "G2_allocations", "G3_sentinel_stale_return",
                                    "G4_sentinel_accuracy", "G5_tests"]) else "FAIL"
    a.output.write_text(json.dumps(res, indent=1) + "\n")
    print(res["verdict"], {g: res[g]["pass"] for g in res if g.startswith("G")})


if __name__ == "__main__":
    main()
