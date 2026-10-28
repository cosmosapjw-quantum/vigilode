#!/usr/bin/env python3
"""Supplemental LP01 eligible-t1 cyclic continuation; never reruns the old pilot.

Uses the frozen runner's oracle, scorer, process ownership/RSS monitor, absolute
clock, cleanup rules and source checks. This file is a separately committed
orchestration change: the prior compiled Rust/runner/scope/binary stay identical.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time
import types

BASE = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve()
RUNNER_PATH = HERE.parents[1]/"src/long_phase_campaign.py"
R = types.ModuleType("lp01_frozen_runner")
R.__file__ = str(RUNNER_PATH)
exec(compile(RUNNER_PATH.read_bytes(), str(RUNNER_PATH), "exec"), R.__dict__)
PRIOR_ATTEMPTS = 81
PRIOR_SECONDS = 38.66
ARMS = tuple((case, method, "1") for case in R.CASES for method in ("full-rhs-dopri5", "lawson"))


def schedule():
    slots = []
    for repeat in range(6):
        offset = 0 if repeat == 0 else repeat-1
        order = ARMS[offset:]+ARMS[:offset]
        for arm in order:
            slots.append({"slot": len(slots), "repeat": repeat, "warmup": repeat == 0,
                          "arm": arm, "arm_id": "ABCDEF"[ARMS.index(arm)]})
    return slots


def slot_disposition(slot, warmup_pass, stop):
    if stop:
        return "NOT_RUN_AFTER_"+stop
    if slot["repeat"] and not warmup_pass.get(slot["arm"], False):
        return "NOT_RUN_FAILED_WARMUP"
    return None


def debit(prior_summary, lp_remaining, shared_remaining, processes_remaining):
    R.require(prior_summary["all_child_process_attempts"] == PRIOR_ATTEMPTS, "prior pilot process debit mismatch")
    observed = prior_summary["elapsed_seconds_including_finalize"]
    R.require(math.isfinite(observed) and observed >= 0, "prior pilot elapsed time invalid")
    # Preserve the frozen 38.66 charge; use any larger actual receipt value too.
    seconds = max(PRIOR_SECONDS, observed)
    R.require(0 < lp_remaining <= 1200-seconds, "LP remaining allowance resets prior elapsed cost")
    R.require(0 < shared_remaining <= 5400-seconds, "shared remaining allowance resets prior elapsed cost")
    R.require(0 < processes_remaining <= 500-PRIOR_ATTEMPTS, "shared process allowance resets prior81 attempts")
    return {"prior_attempts": PRIOR_ATTEMPTS, "prior_seconds_frozen": PRIOR_SECONDS,
            "prior_seconds_observed": observed, "prior_seconds_charged": seconds,
            "lp_remaining_seconds": lp_remaining, "shared_remaining_seconds": shared_remaining,
            "shared_remaining_processes": processes_remaining}


def frozen_inputs(args):
    R.require(R.sha(args.prior_summary) == args.prior_summary_sha256, "prior summary SHA mismatch")
    R.require(R.sha(args.prior_freeze) == args.prior_freeze_sha256, "prior freeze SHA mismatch")
    prior = json.loads(Path(args.prior_summary).read_text())
    old = json.loads(Path(args.prior_freeze).read_text())
    R.require(R.sha(RUNNER_PATH) == args.runner_sha256, "imported runner differs from approved SHA")
    R.require(old["binary_sha256"] == args.binary_sha256, "binary changed from pilot")
    snapshot = R.verify_freeze(args)
    # Verify exact committed harness bytes, including its trailing newline.
    relative = str(HERE.relative_to(BASE))
    command = ["git", "cat-file", "blob", "HEAD:"+relative]
    R.charge_process(command)
    committed = subprocess.check_output(command, cwd=BASE)
    harness_sha = R.sha(HERE)
    R.require(hashlib.sha256(committed).hexdigest() == harness_sha, "supplemental harness is uncommitted or modified")
    snapshot["frozen_file_sha256"][str(HERE)] = harness_sha
    compared = 0
    for path, expected in old["frozen_file_sha256"].items():
        try:
            Path(path).relative_to(BASE)
        except ValueError:
            continue  # Git worktree HEAD/index/ref packaging is intentionally new.
        R.require(snapshot["frozen_file_sha256"].get(path) == expected, "compiled/runner/scope source changed from pilot: "+path)
        compared += 1
    R.require(compared > 0, "prior compiled source identity comparison absent")
    snapshot.update({"prior_binary_source_commit": old["source_commit"],
                     "prior_binary_source_tree": old["source_tree"],
                     "new_orchestration_commit": args.source_commit,
                     "new_orchestration_tree": args.source_tree,
                     "harness_sha256": harness_sha, "runner_sha256": args.runner_sha256,
                     "prior_summary": str(Path(args.prior_summary).resolve()),
                     "prior_summary_sha256": args.prior_summary_sha256,
                     "prior_freeze_sha256": args.prior_freeze_sha256,
                     "unchanged_source_files_compared": compared})
    check_freeze(args, snapshot)
    return prior, snapshot


def check_freeze(args, snapshot):
    R.verify_freeze(args, snapshot)
    # The imported verifier scans its original src directory. Explicitly extend
    # its new-source refusal to this committed supplemental harness directory.
    for path in HERE.parent.rglob("*.py"):
        R.require(str(path.resolve()) in snapshot["frozen_file_sha256"], "unfrozen supplemental analysis source: "+str(path))


def run_slot(slot, args, freeze, deadline, root, environment, counters):
    case, method, tolerance = slot["arm"]
    key = f"slot{slot['slot']:02d}__r{slot['repeat']}__{slot['arm_id']}__{case}__{method}__t1"
    record = {**slot, "row": key, "case": case, "method": method, "tolerance_index": tolerance,
              "timing_authority": "diagnostic_only", "cpu_affinity": freeze["cpu_affinity"],
              "thread_environment": freeze["thread_environment"]}
    stdout = stderr = b""
    stop = None
    try:
        deadline.arm()
        deadline.enter("FREEZE")
        check_freeze(args, freeze)
        R.require(sorted(os.sched_getaffinity(0)) == freeze["cpu_affinity"], "fixed CPU affinity changed")
        R.require(counters["solver_attempts"] < 36, "supplemental solver process cap exhausted")
        command = [str(Path(args.binary).resolve()), "--case", case, "--method", method, "--tolerance-index", "1"]
        deadline.enter("SOLVER")
        R.charge_process(command)
        counters["solver_attempts"] += 1
        record.update({"command": command, "process_cap_seconds": min(90.0, deadline.remaining()),
                       "rss_limit_bytes": 1073741824, "rss_sampling_interval_seconds": .02,
                       "rss_monitor": "imported original process-tree VmRSS monitor"})
        begin = time.monotonic()
        try:
            stdout, stderr, peak, members, code = R.owned_monitored_process(
                command, environment, record["process_cap_seconds"], deadline)
            record.update({"exit_code": code, "status": "SOLVER_EXIT_FAILURE" if code else "OUTPUT_PENDING",
                           "peak_sampled_rss_bytes": peak, "peak_resident_processes": members})
        except R.RSSLimitExceeded as error:
            stdout, stderr = error.stdout, error.stderr
            record.update({"status": "RSS_LIMIT", "exit_code": error.child_returncode,
                           "peak_sampled_rss_bytes": error.peak_rss_bytes,
                           "peak_resident_processes": error.peak_resident_processes})
            R.preserve_owner_failure(record, error)
        except subprocess.TimeoutExpired as error:
            stdout, stderr = error.stdout or b"", error.stderr or b""
            record.update({"status": "TIMEOUT_SOLVER_PROCESS", "exit_code": error.child_returncode,
                           "peak_sampled_rss_bytes": getattr(error, "peak_rss_bytes", None)})
            R.preserve_owner_failure(record, error)
        except OSError as error:
            stdout, stderr = getattr(error, "stdout", b""), getattr(error, "stderr", b"")
            record.update({"status": "RSS_MONITOR_ERROR" if getattr(error, "launched", False) else "LAUNCH_ERROR",
                           "reason": str(error), "exit_code": getattr(error, "child_returncode", None)})
            R.preserve_owner_failure(record, error)
        record["process_elapsed_seconds"] = time.monotonic()-begin
        # Persist actual bytes before expensive reference/freeze/scoring work.
        (root/(key+".stdout")).write_bytes(stdout)
        (root/(key+".stderr")).write_bytes(stderr)
        if R.resume_post_child_freeze(record, deadline):
            check_freeze(args, freeze)
        else:
            stop = "CLEANUP_FAILURE"
            deadline.cancel()
        if record["status"] == "OUTPUT_PENDING":
            deadline.enter("SCORING")
            document = json.loads(stdout)
            scored = R.score(document, case, method, tolerance, deadline)
            record.update({"status": scored["status"], "solve_seconds": document["result"]["solve_seconds"]})
            R.write_new(root/(key+".score.json"), scored)
    except R.DeadlineExpired as error:
        stdout, stderr = getattr(error, "stdout", stdout), getattr(error, "stderr", stderr)
        record["status"] = "TIMEOUT_"+str(error)
        R.preserve_owner_failure(record, error)
        stop = record["status"]
    except R.ProcessBudgetExhausted:
        record["status"] = "NOT_RUN_PROCESS_BUDGET"
        stop = record["status"]
    except (ValueError, KeyError, TypeError, OverflowError, OSError, ImportError) as error:
        stdout, stderr = getattr(error, "stdout", stdout), getattr(error, "stderr", stderr)
        record.update({"status": "RSS_MONITOR_ERROR" if deadline.phase == "SOLVER" else "SCORING_OR_FREEZE_REFUSAL", "reason": str(error)})
        cleanup_failed = R.preserve_owner_failure(record, error)
        if cleanup_failed or deadline.phase == "FREEZE":
            stop = record["status"]
    finally:
        deadline.cancel()
    persist(root, record, stdout, stderr)
    return record, stop


def persist(root, record, stdout=b"", stderr=b""):
    for suffix, data in (("stdout", stdout), ("stderr", stderr)):
        path = root/(record["row"]+"."+suffix)
        if not path.exists():
            path.write_bytes(data)
        record[suffix+"_sha256"] = R.sha(path)
    R.write_new(root/(record["row"]+".receipt.json"), record)


def arm_eligibility(rows):
    eligible = {}
    for i, arm in enumerate(ARMS):
        arm_rows = [row for row in rows if tuple(row["arm"]) == arm]
        eligible["ABCDEF"[i]] = (len(arm_rows) == 6
            and {row["repeat"] for row in arm_rows} == set(range(6))
            and all(row["status"] == "PASS_SAMPLED" for row in arm_rows))
    return eligible


def execute(args):
    # No independent imported campaign() calls: one absolute continuation clock
    # governs every slot, oracle, freeze, and scoring operation.
    R.require(math.isfinite(args.lp_remaining_seconds) and args.lp_remaining_seconds > 0
              and math.isfinite(args.shared_remaining_seconds) and args.shared_remaining_seconds > 0,
              "invalid continuation seconds")
    deadline = R.Deadline(min(args.lp_remaining_seconds, args.shared_remaining_seconds))
    charges = None
    start = time.monotonic()
    account = {"attempts": 0, "cap": args.shared_remaining_processes, "commands": [], "deadline": deadline}
    counters = {"solver_attempts": 0}
    R.PROCESS_ACCOUNTING = account
    root = Path(args.output).resolve()
    root.mkdir(parents=True, exist_ok=False)
    rows = []
    warmup_pass = {}
    stop = None
    allowed = None
    freeze = None
    deadline.arm()
    try:
        # Oracle availability and prior identity/accounting are checked BEFORE
        # starting any solver and remain inside the absolute campaign deadline.
        import mpmath  # noqa: F401
        prior = json.loads(Path(args.prior_summary).read_text())
        charges = debit(prior, args.lp_remaining_seconds, args.shared_remaining_seconds, args.shared_remaining_processes)
        R.check_scope()
        allowed = os.sched_getaffinity(0)
        cpu = min(allowed) if args.cpu is None else args.cpu
        R.require(cpu in allowed, "requested CPU outside allowed affinity")
        os.sched_setaffinity(0, {cpu})
        deadline.enter("FREEZE")
        prior, freeze = frozen_inputs(args)
        freeze["cpu_affinity"] = [cpu]
        freeze["thread_environment"] = {key: "1" for key in ("RAYON_NUM_THREADS", "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS")}
        R.write_new(root/"FREEZE.json", freeze)
        R.write_new(root/"CONTINUATION_CONTRACT.json", {"schedule": schedule(), "charges": charges,
                    "max_new_solvers": 36, "max_new_children_including_initial_git": 43,
                    "old_pilot_is_not_rerun": True, "direct_control": "REUSED_FROM_PRIOR_PILOT"})
    except (R.DeadlineExpired, R.ProcessBudgetExhausted, ValueError, OSError, KeyError, TypeError, OverflowError, ImportError, subprocess.CalledProcessError) as error:
        stop = "TIMEOUT_"+str(error) if isinstance(error, R.DeadlineExpired) else "PREFLIGHT_REFUSAL"
        R.write_new(root/"PREFLIGHT_ERROR.json", {"status": stop, "reason": str(error)})
    finally:
        deadline.cancel()
    try:
        for slot in schedule():
            disposition = slot_disposition(slot, warmup_pass, stop)
            if disposition:
                record = {**slot, "row": f"slot{slot['slot']:02d}__r{slot['repeat']}__{slot['arm_id']}",
                          "status": disposition, "timing_authority": "diagnostic_only"}
                persist(root, record)
            else:
                record, reason = run_slot(slot, args, freeze, deadline, root,
                                          {**os.environ, **freeze["thread_environment"]}, counters)
                if reason:
                    stop = reason
            rows.append(record)
            if slot["warmup"]:
                warmup_pass[slot["arm"]] = record["status"] == "PASS_SAMPLED"
            print(json.dumps({"slot": slot["slot"], "round": slot["repeat"], "arm": slot["arm_id"],
                              "status": record["status"], "new_child_attempts": account["attempts"]}), flush=True)
    finally:
        deadline.cancel()
        R.PROCESS_ACCOUNTING = None
        if allowed is not None:
            os.sched_setaffinity(0, allowed)
    elapsed = time.monotonic()-start
    eligible = arm_eligibility(rows)
    R.write_new(root/"SUMMARY.json", {"schema": "vigilode.lp01-eligible-cyclic.v1", "freeze": freeze,
                "charges": charges, "new_solver_attempts": counters["solver_attempts"],
                "new_child_attempts_including_git": account["attempts"], "new_child_commands": account["commands"],
                "cumulative_child_attempts": PRIOR_ATTEMPTS+account["attempts"],
                "new_elapsed_seconds_including_finalize": elapsed,
                "cumulative_lp_seconds": (charges["prior_seconds_charged"] if charges else PRIOR_SECONDS)+elapsed,
                "stop_reason": stop, "rows": rows, "eligible_arms": eligible,
                "timing_authority": "diagnostic_only", "production_scope": "HOLD; supplemental sampled linear six-arm cohort only"})
    return 0 if all(eligible.values()) else 1


def self_test():
    slots = schedule()
    R.require(len(slots) == 36 and len(ARMS) == 6, "wrong bounded matrix")
    R.require([s["arm_id"] for s in slots[:6]] == list("ABCDEF"), "warmup order changed")
    expected = ("ABCDEF", "BCDEFA", "CDEFAB", "DEFABC", "EFABCD")
    for repeat, order in enumerate(expected, 1):
        R.require("".join(s["arm_id"] for s in slots if s["repeat"] == repeat) == order, "rotation is not round-major")
    for arm in ARMS:
        R.require(sum(s["arm"] == arm for s in slots) == 6, "arm missing warmup/five repeats")
    passed = {arm: True for arm in ARMS}
    failed = {**passed, ARMS[0]: False}
    R.require(sum(slot_disposition(s, failed, None) == "NOT_RUN_FAILED_WARMUP" for s in slots) == 5,
              "failed warmup did not suppress exactly its five timed slots")
    R.require(all(slot_disposition(s, passed, "CLEANUP_FAILURE") == "NOT_RUN_AFTER_CLEANUP_FAILURE" for s in slots),
              "cleanup uncertainty permits another launch")
    R.require(all(slot_disposition(s, passed, None) is None for s in slots), "eligible schedule unexpectedly pruned")
    baseline = {"all_child_process_attempts": 81, "elapsed_seconds_including_finalize": 38.66}
    charges = debit(baseline, 1161.34, 5361.34, 419)
    R.require(charges["prior_attempts"] == 81 and charges["prior_seconds_charged"] == 38.66, "prior debit reset")
    for lp, shared, processes in ((1200, 5361.34, 419), (1161.34, 5400, 419), (1161.34, 5361.34, 500)):
        try:
            debit(baseline, lp, shared, processes)
        except ValueError:
            continue
        raise AssertionError("cumulative resource reset escaped")
    good_rows = [{**s, "status": "PASS_SAMPLED"} for s in slots]
    R.require(all(arm_eligibility(good_rows).values()), "complete passing cohort ineligible")
    R.require(not arm_eligibility(good_rows[1:])["A"], "missing warmup vacuously passes")
    R.require(not any(arm_eligibility([]).values()), "absent arms vacuously pass")
    duplicate = good_rows+[good_rows[0]]
    R.require(not arm_eligibility(duplicate)["A"], "extra/duplicate slot accepted")
    failed_row = [dict(r) for r in good_rows]
    failed_row[12]["status"] = "FAILED_ACCURACY"
    R.require(not arm_eligibility(failed_row)[failed_row[12]["arm_id"]], "failed timed slot selectively omitted")
    print(json.dumps({"status": "PASS", "self_tests": 14, "scheduled_solver_slots": 36,
                      "solver_processes_started": 0, "child_processes_started": 0,
                      "scope": "schedule/failure/accounting tests only"}))
    return 0


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--self-test", action="store_true")
    for name in ("binary", "binary-sha256", "source-commit", "source-tree", "runner-sha256", "prior-summary",
                 "prior-summary-sha256", "prior-freeze", "prior-freeze-sha256", "output"):
        p.add_argument("--"+name)
    p.add_argument("--lp-remaining-seconds", type=float)
    p.add_argument("--shared-remaining-seconds", type=float)
    p.add_argument("--shared-remaining-processes", type=int)
    p.add_argument("--cpu", type=int)
    args = p.parse_args()
    if args.self_test:
        return self_test()
    for key, value in vars(args).items():
        if key not in ("self_test", "cpu"):
            R.require(value is not None, "missing explicit continuation input: "+key)
    return execute(args)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, ImportError, subprocess.CalledProcessError) as error:
        print(json.dumps({"status": "CONTINUATION_PREFLIGHT_REFUSAL", "reason": str(error)}), file=sys.stderr)
        sys.exit(2)
