#!/usr/bin/env python3
"""Replay external adapter outputs against the byte-identical frozen 450 source."""

import argparse
from collections import Counter
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time

import numpy as np

from run_external import SOURCE, CASES, DIMENSIONS, MODES, TOLERANCES, sha256, write_json


FROZEN_ANALYZER_SHA256 = "49a55cc3059d1df8577e9a3bb52c9db639ad90c90cbe1a1102b97097513bc4e6"
FROZEN_VODE_ADAPTER_SHA256 = "3c3951218a4dda6ab0767d0240c4c2bf972cc18a581daa4346f359cb11795e03"
FROZEN_RUNNER_SHA256 = "96e0318a03a4771fd5aa716cbb3fc3d2edc45860b3d9660f5f1f2b506ef52173"
END = {"decay": 4.0, "driven": 4.0, "logistic": 10.0, "harmonic": 12.0,
       "rapid": 2.0, "damped": 4.0, "mixed-scales": 4.0, "nonnormal": 4.0,
       "large-vector": 4.0, "moderate-stiff": 2.0, "stiff-control": 1.0}


def requested_times(case):
    end = END[case]
    return [0.0, end / 4, end / 2, 3 * end / 4, end]


def frozen_analyzer():
    folder = Path(__file__).resolve().parents[1] / "nonstiff_explicit_20261023"
    path = folder / "analyze_bench.py"
    if sha256(path) != FROZEN_ANALYZER_SHA256:
        raise RuntimeError("frozen 450 analyzer byte identity mismatch")
    if sha256(folder / "vode_adapter.py") != FROZEN_VODE_ADAPTER_SHA256:
        raise RuntimeError("frozen 450 problem adapter byte identity mismatch")
    if sha256(folder / "run_bench.py") != FROZEN_RUNNER_SHA256:
        raise RuntimeError("frozen 450 campaign grid byte identity mismatch")
    spec = importlib.util.spec_from_file_location("frozen_nonstiff_analyzer_450", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def expected_grid():
    expected = []
    for case in CASES:
        for dimension in ([4096, 16384] if case == "large-vector" else [DIMENSIONS[case]]):
            for rtol in TOLERANCES:
                for mode in MODES:
                    expected.append({"id": len(expected), "case": case, "dimension": dimension,
                                     "rtol": rtol, "atol": rtol * 1e-3, "mode": mode})
    return expected


def check_record(record, cfg, adapter_commit, analyzer, expected_max_steps=100_000):
    reasons = []
    if not isinstance(record, dict):
        return {"valid": False, "reasons": ["record is not a JSON object"]}
    if record.get("schema") != "vigilode.nonstiff-benchmark-run.v1":
        reasons.append("schema mismatch")
    if record.get("source_commit") != SOURCE:
        reasons.append("frozen problem source mismatch")
    if record.get("adapter_source_commit") != adapter_commit:
        reasons.append("adapter source mismatch")
    for key in ("case", "dimension", "rtol", "atol", "mode"):
        if record.get(key) != cfg[key]:
            reasons.append(f"configuration {key} mismatch")
    expected_times = requested_times(cfg["case"])
    if record.get("requested_times") != expected_times:
        reasons.append("requested time grid mismatch")
    if record.get("end") != END[cfg["case"]]:
        reasons.append("end time mismatch")
    if record.get("max_steps") != expected_max_steps:
        reasons.append("max-step limit mismatch")
    if not isinstance(record.get("success"), bool):
        reasons.append("success must be boolean")
    if not isinstance(record.get("adapter"), dict):
        reasons.append("missing adapter identity")
    else:
        adapter = record["adapter"]
        if not adapter.get("solver") or not adapter.get("method"):
            reasons.append("missing solver or method")
        stiff = cfg["case"] in ("moderate-stiff", "stiff-control")
        expected_method = ("BDF" if stiff else "Tsit45") if cfg["mode"] == "diffsol" else ("Rodas5P" if stiff else "Tsit5")
        expected_solver = "diffsol" if cfg["mode"] == "diffsol" else "OrdinaryDiffEq"
        if adapter.get("solver") != expected_solver or adapter.get("method") != expected_method:
            reasons.append("solver/method differs from frozen contract")
        if adapter.get("adapter_source_commit") != adapter_commit:
            reasons.append("embedded adapter build/source identity mismatch")
        if cfg["mode"] == "diffsol" and adapter.get("version") != "0.16.1":
            reasons.append("diffsol version differs from frozen contract")
        if cfg["mode"] == "julia":
            if adapter.get("julia_version") != "1.12.7":
                reasons.append("Julia runtime differs from frozen contract")
            if record.get("success") is True and adapter.get("preparation_success") is not True:
                reasons.append("same-request Julia JIT preparation failed")
    work = record.get("work")
    if not isinstance(work, dict) or not work:
        reasons.append("missing work counters")
    else:
        def walk(value):
            if isinstance(value, dict):
                for inner in value.values():
                    yield from walk(inner)
            elif isinstance(value, (int, float)) and not isinstance(value, bool):
                yield value
        if any(not math.isfinite(v) or v < 0 for v in walk(work)):
            reasons.append("negative or nonfinite work counter")
        counters = work.get("counters") if isinstance(work.get("counters"), dict) else {}
        rhs = work.get("rhs_attempted", counters.get("rhs_calls"))
        if not isinstance(rhs, (int, float)) or not math.isfinite(rhs) or rhs <= 0:
            reasons.append("no RHS work charged")
    base = analyzer.checked_record(record)
    reasons.extend(base.get("reasons", []))
    if record.get("success"):
        if record.get("times") != expected_times:
            reasons.append("successful saved times mismatch")
        try:
            states = np.asarray(record["states"], dtype=float)
            initial = analyzer.exact(cfg["case"], cfg["dimension"], 0.0)[0]
            if states.shape != (5, cfg["dimension"]) or np.max(np.abs(states[0] - initial)) > 1e-13:
                reasons.append("initial state mismatch")
        except Exception as error:
            reasons.append("initial state check failed: " + repr(error))
    else:
        times = record.get("times")
        states = record.get("states")
        if not isinstance(times, list) or not isinstance(states, list) or len(times) != len(states):
            reasons.append("partial result lacks aligned saved states/times")
        elif not times or times[0] != 0.0 or any(t not in expected_times for t in times):
            reasons.append("partial result lacks accepted initial/observation times")
        else:
            try:
                initial = analyzer.exact(cfg["case"], cfg["dimension"], 0.0)[0]
                first = np.asarray(states[0], dtype=float)
                if first.shape != (cfg["dimension"],) or np.max(np.abs(first - initial)) > 1e-13:
                    reasons.append("partial result initial state mismatch")
            except Exception as error:
                reasons.append("partial initial state check failed: " + repr(error))
        last_time = record.get("last_accepted_time")
        last_state = record.get("last_accepted_state")
        try:
            last_array = np.asarray(last_state, dtype=float)
            if not isinstance(last_time, (int, float)) or not math.isfinite(last_time) or not 0 <= last_time <= END[cfg["case"]]:
                reasons.append("invalid last accepted time")
            elif isinstance(times, list) and times and last_time < times[-1]:
                reasons.append("last accepted time precedes saved observation")
            if last_array.shape != (cfg["dimension"],) or not np.isfinite(last_array).all():
                reasons.append("invalid last accepted state")
        except Exception as error:
            reasons.append("last accepted state check failed: " + repr(error))
    return {"valid": not reasons, "reasons": reasons,
            "max_abs_error": base.get("max_abs_error"),
            "max_normalized_error": base.get("max_normalized_error"),
            "reference_disagreement": base.get("reference_disagreement")}


def verify_campaign(folder, write=True):
    analyzer = frozen_analyzer()
    plan = json.loads((folder / "PLAN.json").read_text())
    receipts = json.loads((folder / "RECEIPTS.json").read_text())
    if plan.get("source_commit") != SOURCE or plan.get("configurations") != expected_grid():
        raise ValueError("campaign has a nonfrozen problem grid")
    if plan.get("per_run_timeout") != 45.0 or plan.get("budget_seconds") != 7200.0:
        raise ValueError("campaign cap mismatch")
    if plan.get("timed_repeats") != 3 or plan.get("warmups_per_configuration") != 1:
        raise ValueError("round count mismatch")
    if len(receipts) != 384:
        raise ValueError(f"expected 384 receipts, observed {len(receipts)}")
    commit = plan["adapter_source_commit"]
    expected = {c["id"]: c for c in plan["configurations"]}
    replay = []
    seen = set()
    for receipt in receipts:
        cfg = expected.get(receipt.get("id"))
        key = (receipt.get("id"), receipt.get("phase"), receipt.get("repeat"))
        reasons = []
        if key in seen:
            reasons.append("duplicate configuration/round")
        seen.add(key)
        if cfg is None or any(receipt.get(k) != cfg[k] for k in ("case", "dimension", "rtol", "atol", "mode")):
            reasons.append("receipt configuration mismatch")
        if key[1:] not in (("warmup", -1), ("timed", 0), ("timed", 1), ("timed", 2)):
            reasons.append("invalid round")
        status = receipt.get("status")
        record = None
        if status in ("COMPLETED", "PROCESS_FAILED"):
            raw = folder / receipt["stdout"]
            if raw.is_file():
                if receipt.get("stdout_sha256") != sha256(raw):
                    reasons.append("stdout hash mismatch")
                try:
                    record = json.loads(raw.read_text())
                except Exception as error:
                    if status == "COMPLETED":
                        reasons.append("malformed completed stdout: " + repr(error))
            elif status == "COMPLETED":
                reasons.append("completed stdout missing")
            stderr = folder / receipt["stderr"]
            if not stderr.is_file() or receipt.get("stderr_sha256") != sha256(stderr):
                reasons.append("stderr missing/hash mismatch")
            if status == "COMPLETED" and receipt.get("exit_code") != 0:
                reasons.append("completed process has nonzero exit")
            if status == "PROCESS_FAILED" and receipt.get("exit_code") in (None, 0):
                reasons.append("failed process lacks nonzero exit")
        elif status not in ("PROCESS_FAILED", "TIMEOUT", "LAUNCH_FAILED", "NOT_RUN_BUDGET"):
            reasons.append("unknown process status")
        if record is not None and cfg is not None:
            checked = check_record(record, cfg, commit, analyzer)
            reasons.extend(checked["reasons"])
            if status == "PROCESS_FAILED" and record.get("success") is True:
                reasons.append("failed process falsely reports success")
        else:
            checked = {}
        replay.append({"id": receipt.get("id"), "phase": receipt.get("phase"),
                       "repeat": receipt.get("repeat"), "status": status,
                       "stdout": receipt.get("stdout"), "record_success": record.get("success") if record else None,
                       "valid": not reasons and record is not None, "reasons": reasons,
                       "max_abs_error": checked.get("max_abs_error"),
                       "max_normalized_error": checked.get("max_normalized_error"),
                       "reference_disagreement": checked.get("reference_disagreement")})
    expected_keys = {(c["id"], phase, repeat) for c in expected.values()
                     for phase, repeat in (("warmup", -1), ("timed", 0), ("timed", 1), ("timed", 2))}
    missing = sorted(expected_keys - seen)
    summary = {"source_commit": SOURCE, "adapter_source_commit": commit,
               "frozen_analyzer_sha256": FROZEN_ANALYZER_SHA256,
               "receipt_count": len(receipts), "missing_rounds": missing,
               "process_status": dict(Counter(r.get("status") for r in receipts)),
               "valid_replays": sum(r["valid"] for r in replay),
               "invalid_replays": sum(not r["valid"] for r in replay),
               "successful_valid_replays": sum(r["valid"] and r["record_success"] for r in replay)}
    if write:
        write_json(folder / "ADAPTER_REPLAY.json", replay)
        write_json(folder / "ADAPTER_REPLAY_SUMMARY.json", summary)
    return summary, replay


def check_negative_control(kind, status, record, request, adapter_commit, analyzer):
    """A bounded refusal may exit nonzero; exhaustion must return honest JSON."""
    reasons = []
    if status in ("TIMEOUT", "LAUNCH_FAILED"):
        reasons.append("negative control timed out or failed launch")
    if kind == "max-step-exhaustion":
        if not isinstance(record, dict):
            return [*reasons, "step exhaustion lacks parseable failure record"]
        checked = check_record(record, request, adapter_commit, analyzer,
                               expected_max_steps=1)
        reasons.extend(checked["reasons"])
        if record.get("success") is not False:
            reasons.append("step exhaustion did not report failure")
        if not isinstance(record.get("times"), list) or len(record["times"]) >= 5:
            reasons.append("step exhaustion did not retain a partial observation history")
        if not isinstance(record.get("message"), str) or not record["message"].strip():
            reasons.append("step exhaustion lacks failure reason")
        return reasons
    if status == "PROCESS_FAILED":
        if isinstance(record, dict) and record.get("success") is True:
            reasons.append("failed process falsely reports accepted invalid input")
        return reasons
    if status != "COMPLETED" or not isinstance(record, dict):
        return [*reasons, "invalid input lacks bounded refusal"]
    if record.get("success") is not False:
        reasons.append("negative control was accepted")
    if record.get("schema") != "vigilode.nonstiff-benchmark-run.v1":
        reasons.append("explicit refusal schema mismatch")
    for key in ("case", "dimension", "rtol", "atol", "mode", "source_commit", "adapter_source_commit", "max_steps"):
        if record.get(key) != request[key]:
            reasons.append(f"explicit refusal {key} mismatch")
    if not isinstance(record.get("message"), str) or not record["message"].strip():
        reasons.append("explicit refusal lacks reason")
    return reasons


def preflight(diffsol, julia, adapter_commit, folder):
    """Exercise all frozen case families and typed refusals before the campaign."""
    if folder.exists():
        raise FileExistsError("preflight output already exists")
    adapters = {"diffsol": diffsol.resolve(), "julia": julia.resolve()}
    for mode, path in adapters.items():
        if not path.is_file() or not os.access(path, os.X_OK):
            raise FileNotFoundError(f"{mode} adapter is not executable: {path}")
    analyzer = frozen_analyzer()
    folder.mkdir(parents=True)
    jobs = []
    for case in CASES:
        for dimension in ([4096, 16384] if case == "large-vector" else [DIMENSIONS[case]]):
            for mode in MODES:
                jobs.append(("positive", {"case": case, "dimension": dimension,
                                          "rtol": 1e-5, "atol": 1e-8, "mode": mode,
                                          "source_commit": SOURCE,
                                          "adapter_source_commit": adapter_commit,
                                          "max_steps": 100_000}))
    for mode in MODES:
        template = {"case": "harmonic", "dimension": 8, "rtol": 1e-5, "atol": 1e-8,
                    "mode": mode, "source_commit": SOURCE,
                    "adapter_source_commit": adapter_commit, "max_steps": 100_000}
        for label, changes in (
            ("unknown-family", {"case": "unregistered-family"}),
            ("odd-oscillator", {"dimension": 7}),
            ("invalid-tolerance", {"rtol": -1e-5}),
            ("max-step-exhaustion", {"case": "stiff-control", "dimension": 12,
                                     "rtol": 1e-9, "atol": 1e-12, "max_steps": 1}),
        ):
            jobs.append((label, {**template, **changes}))
    write_json(folder / "PLAN.json", {"source_commit": SOURCE,
                                      "adapter_source_commit": adapter_commit,
                                      "frozen_analyzer_sha256": FROZEN_ANALYZER_SHA256,
                                      "per_process_timeout": 45.0,
                                      "jobs": [{"kind": kind, "request": req} for kind, req in jobs],
                                      "adapters": {k: {"binary": str(v), "sha256": sha256(v)}
                                                   for k, v in adapters.items()}})
    receipts = []
    for index, (kind, request) in enumerate(jobs):
        stdout = folder / f"{index:03d}.stdout.json"
        stderr = folder / f"{index:03d}.stderr.log"
        start = time.monotonic()
        try:
            with stdout.open("w") as out, stderr.open("w") as err:
                result = subprocess.run([str(adapters[request["mode"]])], input=json.dumps(request),
                                        text=True, stdout=out, stderr=err, check=False, timeout=45.0,
                                        env={**os.environ, "VIGILODE_ADAPTER_SOURCE_COMMIT": adapter_commit})
            status, code = "COMPLETED" if result.returncode == 0 else "PROCESS_FAILED", result.returncode
        except subprocess.TimeoutExpired:
            status, code = "TIMEOUT", None
        except OSError as error:
            stderr.write_text(str(error) + "\n")
            status, code = "LAUNCH_FAILED", None
        receipt = {"index": index, "kind": kind, "request": request,
                   "status": status, "exit_code": code,
                   "elapsed_process_seconds": time.monotonic() - start,
                   "stdout": stdout.name, "stderr": stderr.name,
                   "stdout_sha256": sha256(stdout) if stdout.exists() else None,
                   "stderr_sha256": sha256(stderr) if stderr.exists() else None}
        reasons = []
        try:
            record = json.loads(stdout.read_text())
        except Exception as error:
            record = None
            if kind == "positive":
                reasons.append("missing or malformed positive result: " + repr(error))
        if kind == "positive":
            if status != "COMPLETED":
                reasons.append("positive process did not complete")
            if record is not None:
                checked = check_record(record, request, adapter_commit, analyzer)
                reasons.extend(checked["reasons"])
                if record.get("success") is not True:
                    reasons.append("representative solve unsuccessful")
        else:
            reasons.extend(check_negative_control(kind, status, record, request,
                                                  adapter_commit, analyzer))
        receipt["valid"] = not reasons
        receipt["reasons"] = reasons
        receipts.append(receipt)
        write_json(folder / "RECEIPTS.json", receipts)
        print(json.dumps({"index": index, "kind": kind, "mode": request["mode"],
                          "case": request["case"], "status": status, "valid": not reasons}), flush=True)
    result = {"planned": len(jobs), "valid": sum(r["valid"] for r in receipts),
              "failed": sum(not r["valid"] for r in receipts),
              "case_mode_pairs": len([r for r in receipts if r["kind"] == "positive" and r["valid"]]),
              "all_family_saved_state_replay": all(r["valid"] for r in receipts if r["kind"] == "positive")}
    write_json(folder / "SUMMARY.json", result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("campaign", type=Path, nargs="?")
    parser.add_argument("--preflight-out", type=Path)
    parser.add_argument("--diffsol-adapter", type=Path)
    parser.add_argument("--julia-adapter", type=Path)
    parser.add_argument("--adapter-source-commit")
    args = parser.parse_args()
    if args.preflight_out:
        if not all((args.diffsol_adapter, args.julia_adapter, args.adapter_source_commit)):
            parser.error("preflight requires both adapters and adapter source commit")
        summary = preflight(args.diffsol_adapter, args.julia_adapter,
                            args.adapter_source_commit, args.preflight_out)
        print(json.dumps(summary, indent=2))
        return 0 if not summary["failed"] else 1
    if args.campaign is None:
        parser.error("campaign directory required")
    summary, _ = verify_campaign(args.campaign)
    print(json.dumps(summary, indent=2))
    return 0 if not summary["missing_rounds"] and summary["invalid_replays"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
