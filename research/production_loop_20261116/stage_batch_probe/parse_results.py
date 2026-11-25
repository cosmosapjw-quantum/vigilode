#!/usr/bin/env python3
"""Node V32 (owner decision OD-L1-28): parse the raw outputs of this folder into RESULTS.json and
render REPORT_KO.md (Korean). Every number in both files comes from the raw outputs:

- logs/<batch>/<binary>/<condition>/: the CLI reports of provoke.sh (reports.tar.xz after
  pack_logs.py, loose run_NNN.json before) and runs.tsv (exit status, start/end, load average);
- logs/tests/*.log: the test runs of the node's tests against the old and the new probe, and the
  repeated runs of the late-worker test executables (repeat_tests.sh);
- logs/checks/*.log: the step 6 checks (run_checks.sh);
- crates/rodas5p-integrators/src/stage_batch.rs: the rendezvous constants (bound, poll interval).

Wall times are diagnostic only; no verdict depends on them.

usage: python3 -I parse_results.py      (from any directory)
"""
import collections
import hashlib
import json
import math
import os
import re
import struct
import subprocess
import statistics
import tarfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
LOGS = os.path.join(HERE, "logs")
SOURCE = os.path.join(ROOT, "crates", "rodas5p-integrators", "src", "stage_batch.rs")
SMOKE_CHECKSUM = "4079849b6d3c0702f14e6dfb977319f6f1f6891c046786a8bd7aff1f1d39c950"
HOSTED_FALSE_CHECKSUM = "577161ebffb8eaf60ff44b02624e192d18853d72bff697bb34c27aa8ba52d6ea"
OLD_BLOB = "c2f44c327c53d7c6dfe7ba80b26cd43486d56b43"
TASK_TEXT_CONDITIONS = ("strace1", "strace2")
CONDITION_TEXT = {
    "strace1": "taskset -c 8 strace -f -qq -e trace=none (task text)",
    "strace2": "taskset -c 8,9 strace -f -qq -e trace=none (task text)",
    "hog1": "strace1 + 2 busy loops started on CPU 8 per run",
    "hog4": "strace1 + 4 busy loops started on CPU 8 per run",
    "hogonly1": "taskset -c 8 + 2 busy loops started on CPU 8 per run, no strace",
    "shared1": "one strace -f on CPU 8 traces 4 concurrent runs",
}
# Busy loops started per run (provoke.sh with_hogs). Each lives HOG_SECONDS: the kill after the run
# reaches only the wrapper subshell, not `timeout 30 sh -c 'while :; do :; done'`, so loops of
# consecutive runs accumulate on CPU 8. The live count at a run's start is rebuilt from runs.tsv.
HOGS_PER_RUN = {"hog1": 2, "hog4": 4, "hogonly1": 2}
HOG_SECONDS = 30.0


def bits(value):
    return struct.unpack("<Q", struct.pack("<d", float(value)))[0]


def recomposed(report):
    """scientific_checksum of src/stage_batch.rs, rebuilt from the report fields."""
    text = lambda flag: "true" if flag else "false"
    lines = [f"parallel-observed={text(report['stage_parallelism_observed'])}\n"]
    for case in report["cases"]:
        lines.append(f"case|{case['case_id']}|{case['family']}|{case['dimension']}|"
                     f"{case['work_repeats']}|{case['stages']}\n")
    for row in report["rows"]:
        lines.append(f"row|{row['case_id']}|{row['kernel']}|{row['backend']}|{row['threads']}|"
                     f"{row['stages']}|{row['dimension']}|{row['work_repeats']}|"
                     f"{bits(row['max_abs_difference']):016x}|{text(row['actual_stage_parallel'])}\n")
    return hashlib.sha256("".join(lines).encode()).hexdigest()


def read_reports(directory):
    """{run index: report bytes} from reports.tar.xz and loose run_NNN.json files."""
    blobs = {}
    archive = os.path.join(directory, "reports.tar.xz")
    if os.path.exists(archive):
        with tarfile.open(archive, "r:xz") as tar:
            for member in tar.getmembers():
                if member.name.endswith(".json"):
                    blobs[member.name] = tar.extractfile(member).read()
    for name in os.listdir(directory):
        if name.startswith("run_") and name.endswith(".json"):
            if name in blobs:
                raise SystemExit(f"{directory}: {name} both packed and loose")
            with open(os.path.join(directory, name), "rb") as handle:
                blobs[name] = handle.read()
    return {int(re.match(r"run_(\d+)\.json", n).group(1)): b for n, b in blobs.items()}


def read_runs_tsv(directory):
    rows = {}
    with open(os.path.join(directory, "runs.tsv"), encoding="utf-8") as handle:
        next(handle)
        for line in handle:
            index, status, start, end, load = line.rstrip("\n").split("\t")
            rows[int(index)] = {"exit": int(status), "start": float(start),
                                "wall_seconds": float(end) - float(start),
                                "loadavg_1min": float(load.split()[0])}
    return rows


def all_runs():
    """[(batch, binary, condition, index, runs.tsv row)] of every provocation run."""
    out = []
    for batch in sorted(d for d in os.listdir(LOGS) if re.fullmatch(r"b\d+\w*", d)):
        for binary in sorted(os.listdir(os.path.join(LOGS, batch))):
            binary_dir = os.path.join(LOGS, batch, binary)
            if not os.path.isdir(binary_dir):
                continue
            for condition in sorted(os.listdir(binary_dir)):
                directory = os.path.join(binary_dir, condition)
                for index, meta in read_runs_tsv(directory).items():
                    out.append((batch, binary, condition, index, meta))
    return out


def live_busy_loops():
    """{(batch, binary, condition, index): busy loops alive on CPU 8 when the run started}."""
    runs = all_runs()
    starts = sorted((meta["start"], HOGS_PER_RUN.get(condition, 0))
                    for _, _, condition, _, meta in runs if HOGS_PER_RUN.get(condition, 0))
    live = {}
    for batch, binary, condition, index, meta in runs:
        t = meta["start"]
        live[(batch, binary, condition, index)] = sum(
            count for start, count in starts if 0.0 <= t - start < HOG_SECONDS)
    return live


def quantiles(values):
    if not values:
        return None
    ordered = sorted(values)
    pick = lambda q: ordered[min(len(ordered) - 1, int(q * (len(ordered) - 1) + 0.5))]
    return {"min": round(ordered[0], 4), "median": round(statistics.median(ordered), 4),
            "p90": round(pick(0.9), 4), "max": round(ordered[-1], 4)}


def provocation():
    cells = collections.defaultdict(lambda: {
        "runs": 0, "exit_nonzero": 0, "missing_report": 0, "parallel_observed_false": 0,
        "checksums": collections.Counter(), "observed_max_parallel_tasks": collections.Counter(),
        "composition_mismatches": 0, "wall": [], "loadavg_1min": [], "busy_loops": [],
        "batches": []})
    live = live_busy_loops()
    failing = []
    per_batch = collections.defaultdict(lambda: {"runs": 0, "parallel_observed_false": 0})
    batches = sorted(d for d in os.listdir(LOGS) if re.fullmatch(r"b\d+\w*", d))
    for batch in batches:
        for binary in sorted(os.listdir(os.path.join(LOGS, batch))):
            binary_dir = os.path.join(LOGS, batch, binary)
            if not os.path.isdir(binary_dir):
                continue
            for condition in sorted(os.listdir(binary_dir)):
                directory = os.path.join(binary_dir, condition)
                reports = read_reports(directory)
                runs = read_runs_tsv(directory)
                cell = cells[(binary, condition)]
                cell["batches"].append(f"{batch}:{len(runs)}")
                for index, meta in sorted(runs.items()):
                    per_batch[(batch, binary, condition)]["runs"] += 1
                    cell["runs"] += 1
                    cell["wall"].append(meta["wall_seconds"])
                    cell["loadavg_1min"].append(meta["loadavg_1min"])
                    busy = live[(batch, binary, condition, index)]
                    cell["busy_loops"].append(busy)
                    if meta["exit"] != 0:
                        cell["exit_nonzero"] += 1
                    if index not in reports:
                        cell["missing_report"] += 1
                        continue
                    report = json.loads(reports[index])
                    cell["checksums"][report["scientific_checksum"]] += 1
                    cell["observed_max_parallel_tasks"][report["observed_max_parallel_tasks"]] += 1
                    if recomposed(report) != report["scientific_checksum"]:
                        cell["composition_mismatches"] += 1
                    if not report["stage_parallelism_observed"]:
                        cell["parallel_observed_false"] += 1
                        per_batch[(batch, binary, condition)]["parallel_observed_false"] += 1
                        failing.append({"batch": batch, "binary": binary, "condition": condition,
                                        "run": index, "busy_loops_on_cpu8_at_start": busy,
                                        "observed_max_parallel_tasks": report["observed_max_parallel_tasks"],
                                        "verdict": report["verdict"],
                                        "scientific_checksum": report["scientific_checksum"]})
    table = {}
    for (binary, condition), cell in sorted(cells.items()):
        table.setdefault(binary, {})[condition] = {
            "condition": CONDITION_TEXT.get(condition, condition),
            "batches": cell["batches"],
            "runs": cell["runs"],
            "exit_nonzero": cell["exit_nonzero"],
            "missing_report": cell["missing_report"],
            "parallel_observed_false": cell["parallel_observed_false"],
            "checksums": dict(cell["checksums"]),
            "composition_mismatches": cell["composition_mismatches"],
            "observed_max_parallel_tasks": {str(k): v for k, v in
                                            sorted(cell["observed_max_parallel_tasks"].items())},
            "busy_loops_on_cpu8_at_start": quantiles(cell["busy_loops"]),
            "wall_seconds_diagnostic": quantiles(cell["wall"]),
            "loadavg_1min_at_start": quantiles(cell["loadavg_1min"]),
        }
    return batches, table, failing, dict(per_batch)


def fisher_one_sided(a_false, a_runs, b_false, b_runs):
    """P(X >= a_false) for the false runs of A under equal rates (hypergeometric)."""
    total, false = a_runs + b_runs, a_false + b_false
    denominator = math.comb(total, a_runs)
    return sum(math.comb(false, k) * math.comb(total - false, a_runs - k)
               for k in range(a_false, min(false, a_runs) + 1)) / denominator


def libtest(text):
    results = [{"passed": int(m[0]), "failed": int(m[1]), "ignored": int(m[2])} for m in
               re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", text)]
    tests = re.findall(r"^test (\S+) \.\.\. (ok|FAILED|ignored)", text, re.M)
    running = re.findall(r"Running (\S+) \(", text)
    return {"binaries": running, "results": results,
            "passed": sum(r["passed"] for r in results), "failed": sum(r["failed"] for r in results),
            "failed_tests": [name for name, status in tests if status == "FAILED"]}


def read(path):
    with open(path, encoding="utf-8", errors="replace") as handle:
        return handle.read()


def node_tests():
    out = {}
    for label in ("old_code_new_tests", "new_code_new_tests"):
        path = os.path.join(LOGS, "tests", f"{label}.log")
        if not os.path.exists(path):
            continue
        text = read(path)
        blob = re.search(r"stage_batch\.rs blob ([0-9a-f]{40})", text).group(1)
        late = re.findall(r"repeat=(\d+) late_workers=(\d+) observed_max_parallel_tasks=(\d+) "
                          r"checksum=([0-9a-f]{64})", text)
        out[label] = {"stage_batch_blob": blob, "exit": int(re.search(r"^exit=(\d+)", text, re.M).group(1)),
                      **libtest(text),
                      "contracts_line": re.findall(r"^repeats=.*$", text, re.M),
                      "late_worker_repeats": [{"repeat": int(r), "late_workers": int(l),
                                               "observed_max_parallel_tasks": int(m), "checksum": c}
                                              for r, l, m, c in late],
                      "panics": re.findall(r"panicked at .*\n(.*)", text)}
    return out


def late_worker_repeats():
    path = os.path.join(LOGS, "tests", "late_workers_repeat.log")
    if not os.path.exists(path):
        return None
    text = read(path)
    executables = dict(re.findall(r"^# executable (\w+)=\S+ sha256=([0-9a-f]{64})", text, re.M))
    summary = {}
    for label, status in re.findall(r"^## run=\d+ label=(\w+) exit=(\d+)", text, re.M):
        entry = summary.setdefault(label, {"runs": 0, "exit_zero": 0, "exit_nonzero": 0,
                                           "executable_sha256": executables.get(label)})
        entry["runs"] += 1
        entry["exit_zero" if status == "0" else "exit_nonzero"] += 1
    summary.setdefault("old", {})["panic_observed_1"] = len(
        re.findall(r"probe observed 1 simultaneously active task\(s\) with late workers", text))
    new_lines = re.findall(r"repeat=\d+ late_workers=(\d+) observed_max_parallel_tasks=(\d+) "
                           r"checksum=([0-9a-f]{64})", text)
    summary.setdefault("new", {}).update({
        "reports": len(new_lines),
        "late_workers": dict(collections.Counter(int(l) for l, _, _ in new_lines)),
        "observed_max_parallel_tasks": dict(collections.Counter(int(m) for _, m, _ in new_lines)),
        "checksums": dict(collections.Counter(c for _, _, c in new_lines))})
    return summary


def latency():
    """Rendezvous waits on fresh pools (latency.sh, ignored test stage_batch_probe_latency)."""
    out = {}
    directory = os.path.join(LOGS, "latency")
    if not os.path.isdir(directory):
        return out
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".log"):
            continue
        text = read(os.path.join(directory, name))
        line = re.search(r"diagnostic pools=(\d+) second_task_never=(\d+) wait_seconds min=([\d.]+) "
                         r"median=([\d.]+) p90=([\d.]+) p99=([\d.]+) max=([\d.]+)", text)
        status = re.search(r"^exit=(\d+)", text, re.M)
        entry = {"exit": int(status.group(1)) if status else None,
                 "header": re.search(r"^# condition .*$", text, re.M).group(0)}
        if line:
            keys = ["pools", "second_task_never", "min", "median", "p90", "p99", "max"]
            values = [int(line.group(1)), int(line.group(2))] + [float(v) for v in line.groups()[2:]]
            entry.update(dict(zip(keys, values)))
            entry["largest"] = [float(v) for v in re.findall(r"diagnostic largest\[\d+\]=([\d.]+)", text)]
        out[name[:-4]] = entry
    return out


def checks():
    out = {}
    directory = os.path.join(LOGS, "checks")
    if not os.path.isdir(directory):
        return out
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".log"):
            continue
        text = read(os.path.join(directory, name))
        status = re.search(r"^exit=(\d+)", text, re.M)
        entry = {"command": re.search(r"^# command: (.*)$", text, re.M).group(1),
                 "head": re.search(r"^# head (\S+)", text, re.M).group(1),
                 "exit": int(status.group(1)) if status else None}
        if " test " in f" {entry['command']} ":
            entry.update(libtest(text))
        if name == "clippy.log":
            entry["warnings"] = len(re.findall(r"^warning", text, re.M))
            entry["errors"] = len(re.findall(r"^error", text, re.M))
        if name.startswith("loop_checker"):
            entry["output"] = [line for line in text.splitlines() if not line.startswith("#")
                               and not line.startswith("exit=")]
        out[name[:-4]] = entry
    return out


def code():
    source = read(SOURCE)
    bound = re.search(r"RENDEZVOUS_BOUND: Duration = Duration::from_(secs|millis)\((\d+)\)", source)
    poll = re.search(r"RENDEZVOUS_POLL: Duration = Duration::from_(micros|millis)\((\d+)\)", source)
    blob = subprocess.run(["git", "-C", ROOT, "hash-object", SOURCE], capture_output=True,
                          text=True, check=True).stdout.strip()
    head = subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True,
                          text=True, check=True).stdout.strip()
    scale = {"secs": 1.0, "millis": 1e-3, "micros": 1e-6}
    return {"head_when_parsed": head, "stage_batch_blob_before": OLD_BLOB,
            "stage_batch_blob_after": blob,
            "rendezvous_bound_seconds": int(bound.group(2)) * scale[bound.group(1)] if bound else None,
            "rendezvous_poll_seconds": int(poll.group(2)) * scale[poll.group(1)] if poll else None}


def totals(table, binary, conditions=None):
    cells = [c for name, c in table.get(binary, {}).items() if conditions is None or name in conditions]
    return {"runs": sum(c["runs"] for c in cells),
            "parallel_observed_false": sum(c["parallel_observed_false"] for c in cells),
            "exit_nonzero": sum(c["exit_nonzero"] for c in cells),
            "missing_report": sum(c["missing_report"] for c in cells)}


def discriminators(table, failing, tests, repeats, check_results):
    old_all, new_all = totals(table, "old"), totals(table, "new")
    old_task = totals(table, "old", TASK_TEXT_CONDITIONS)
    provoking = sorted({f["condition"] for f in failing if f["binary"] == "old"})
    old_late = (repeats or {}).get("old", {})
    new_late = (repeats or {}).get("new", {})
    out = []
    provoked = old_all["parallel_observed_false"] > 0 or old_late.get("exit_nonzero", 0) > 0
    out.append({
        "name": "old failure provoked",
        "frozen_threshold": "PASS if any run of the old probe reports stage_parallelism_observed=false "
                            "under a provocation; NOT_DECIDABLE if none does",
        "measured": (f"CLI, task-text conditions {'/'.join(TASK_TEXT_CONDITIONS)}: "
                     f"{old_task['parallel_observed_false']}/{old_task['runs']} false; all CLI conditions: "
                     f"{old_all['parallel_observed_false']}/{old_all['runs']} false (in {', '.join(provoking) or 'none'}), "
                     f"each with checksum {sorted({f['scientific_checksum'][:8] for f in failing if f['binary'] == 'old'})}; "
                     f"late-worker test executable: {old_late.get('exit_nonzero', 0)}/{old_late.get('runs', 0)} runs failed "
                     f"with 'probe observed 1'"),
        "verdict": "PASS" if provoked else "NOT_DECIDABLE",
    })
    new_ok = (new_all["parallel_observed_false"] == 0 and new_all["runs"] > 0
              and new_late.get("exit_nonzero", 1) == 0)
    out.append({
        "name": "new code never shows the failure under the same provocation",
        "frozen_threshold": "0 runs of the new probe with stage_parallelism_observed=false in every "
                            "provocation condition and every late-worker test run",
        "measured": (f"CLI: {new_all['parallel_observed_false']}/{new_all['runs']} false (interleaved with the "
                     f"old binary in batch b2); late-worker test executable: {new_late.get('exit_zero', 0)}/"
                     f"{new_late.get('runs', 0)} runs passed"),
        "verdict": "PASS" if new_ok else "FAIL",
    })
    new_checksums = collections.Counter()
    for cell in table.get("new", {}).values():
        new_checksums.update(cell["checksums"])
    old_true = collections.Counter()
    for cell in table.get("old", {}).values():
        old_true.update({k: v for k, v in cell["checksums"].items() if k != HOSTED_FALSE_CHECKSUM})
    mismatches = sum(c["composition_mismatches"] for b in table.values() for c in b.values())
    test_checksums = set(new_late.get("checksums", {}))
    for line in tests.get("new_code_new_tests", {}).get("contracts_line", []):
        test_checksums.update(re.findall(r"checksum=([0-9a-f]{64})", line))
    unchanged = (set(new_checksums) == {SMOKE_CHECKSUM} and set(old_true) == {SMOKE_CHECKSUM}
                 and mismatches == 0 and test_checksums == {SMOKE_CHECKSUM})
    out.append({
        "name": "checksum unchanged",
        "frozen_threshold": f"every new-probe smoke checksum (CLI runs and tests) equals {SMOKE_CHECKSUM}; "
                            "every old-probe run with parallel-observed=true has the same value; the "
                            "composition rebuilt from each report equals its checksum",
        "measured": (f"new CLI checksums {dict(new_checksums)}; old CLI checksums other than the "
                     f"false value {dict(old_true)}; test checksums {sorted(test_checksums)}; "
                     f"composition mismatches {mismatches}"),
        "verdict": "PASS" if unchanged else "FAIL",
    })
    suite_names = ["cli_contracts", "integrators_stage_batch_tests", "clippy", "fmt"]
    present = [n for n in suite_names if n in check_results]
    suites_ok = len(present) == len(suite_names) and all(check_results[n]["exit"] == 0 for n in present)
    out.append({
        "name": "suites",
        "frozen_threshold": "cli_contracts, the stage_batch test targets (stage_batch_contracts, "
                            "generic_scope_contracts, the two new ones), clippy -D warnings and fmt exit 0",
        "measured": "; ".join(
            f"{n}: exit {check_results[n]['exit']}"
            + (f", {check_results[n].get('passed')} passed/{check_results[n].get('failed')} failed"
               if "passed" in check_results[n] else "")
            for n in present) + ("" if len(present) == len(suite_names) else
                                 f"; not run: {sorted(set(suite_names) - set(present))}"),
        "verdict": "PASS" if suites_ok else ("NOT_RUN" if len(present) < len(suite_names) else "FAIL"),
    })
    return out


def number(value, spec):
    return "n/a" if value is None else format(value, spec)


def fmt_table(rows):
    lines = ["| " + " | ".join(rows[0]) + " |", "|" + "---|" * len(rows[0])]
    lines += ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows[1:]]
    return "\n".join(lines)


def render(results):
    p = results["provocation"]
    code_info = results["code"]
    verdict_ko = {"PASS": "통과", "FAIL": "실패", "NOT_DECIDABLE": "판정 불가", "NOT_RUN": "실행 안 함"}
    rows = [["binary", "조건", "batch:실행", "실행", "false", "관측 최대 동시 작업 분포", "checksum",
             "CPU 8의 바쁜 loop 수 (시작 시, 중앙값 [최소, 최대])", "wall 중앙값 s (진단)"]]
    for binary in ("old", "new"):
        for condition, cell in p["by_binary_condition"].get(binary, {}).items():
            busy = cell["busy_loops_on_cpu8_at_start"] or {}
            rows.append([binary, f"{condition}: {cell['condition']}", ", ".join(cell["batches"]),
                         cell["runs"], cell["parallel_observed_false"],
                         ", ".join(f"{k}: {v}" for k, v in cell["observed_max_parallel_tasks"].items()),
                         ", ".join(f"{k[:8]}… {v}" for k, v in cell["checksums"].items()),
                         f"{busy.get('median')} [{busy.get('min')}, {busy.get('max')}]",
                         (cell["wall_seconds_diagnostic"] or {}).get("median")])
    failing = [["batch", "binary", "조건", "run", "관측 최대", "CPU 8의 바쁜 loop 수", "checksum"]] + [
        [f["batch"], f["binary"], f["condition"], f["run"], f["observed_max_parallel_tasks"],
         f["busy_loops_on_cpu8_at_start"], f["scientific_checksum"][:8] + "…"] for f in p["failing_runs"]]
    t = results["node_tests"]
    rep = results["late_worker_test_repeats"] or {}
    checks_rows = [["검사", "명령", "exit", "결과"]]
    for name, entry in results["checks"].items():
        detail = (f"{entry.get('passed')} passed, {entry.get('failed')} failed" if "passed" in entry
                  else (" / ".join(entry.get("output", [])) if name.startswith("loop_checker") else ""))
        checks_rows.append([name, f"`{entry['command']}`", entry["exit"], detail])
    disc_rows = [["판정 항목", "측정", "판정"]] + [
        [d["name"], d["measured"], verdict_ko.get(d["verdict"], d["verdict"])] for d in results["discriminators"]]
    stat = results["statistics"]
    lines = [
        "# V32 stage-batch 동시성 probe: 상한 있는 rendezvous (OD-L1-28)",
        "",
        f"이 문서는 `parse_results.py`가 `logs/`의 원 출력에서 만든다. 숫자는 모두 그 출력에서 읽었다. "
        f"wall time은 진단값이며 어떤 판정에도 쓰지 않는다. 기준 commit d4e2772f, 파싱 시점 head "
        f"`{code_info['head_when_parsed'][:8]}`.",
        "",
        "## 1. 원인",
        "",
        "`run_concurrency_probe`는 새 local rayon pool(4 thread)을 만들고 곧바로 작업 8개(각 1 ms sleep)를 넣는다. "
        "rayon의 `build`는 worker가 시작되기를 기다리지 않는다. 먼저 돈 worker가 작업 전체를 받아 자기 deque에 나누고, "
        "다른 worker는 실행된 뒤에야 훔쳐 갈 수 있다. 다른 worker가 약 8 ms 안에 돌지 못하면(추적기 아래의 늦은 thread "
        "시작, 부하) 한 worker가 8개를 모두 혼자 실행하고 최대 동시 작업 수는 1, `parallel-observed=false`가 되어 "
        f"checksum이 `{HOSTED_FALSE_CHECKSUM[:8]}…`(hosted 두 번째 실행 값)으로 바뀐다.",
        "",
        "## 2. 변경",
        "",
        f"각 작업은 활성으로 표시한 뒤, 동시에 활성인 작업이 2개 이상 관측되었거나 마감이 지날 때까지 "
        f"{code_info['rendezvous_poll_seconds'] * 1e6:.0f} µs씩 잠들며 기다린다. 마감은 첫 작업이 도착한 시각 + "
        f"{code_info['rendezvous_bound_seconds']:g} s로 한 번 정해지고 모든 작업이 공유한다. 두 작업을 동시에 돌릴 수 "
        "없는 pool은 한 번만 기다린 뒤 지금처럼 1을 보고한다. checksum 구성, 보고서 schema, 작업 8개 × 1 ms, 4 thread "
        f"pool은 그대로다. `stage_batch.rs` blob `{code_info['stage_batch_blob_before'][:8]}` → "
        f"`{code_info['stage_batch_blob_after'][:8]}` (CHECKPOINT 재고정은 OD-L1-28에 따라 orchestrator가 한다).",
        "",
        "## 3. 판정",
        "",
        fmt_table(disc_rows),
        "",
        "## 4. CLI 유발 실험 (`provoke.sh`)",
        "",
        "b1, b1h는 이전 binary만, b2는 같은 조건에서 이전/새 binary를 run마다 번갈아 실행했다(같은 시각의 host 부하를 "
        "함께 받는다). CPU 8(과 9)에 고정했다. 과제 문장의 조건(strace1, strace2)만으로는 실패가 나오지 않아 "
        "추적기(strace)와 같은 CPU에 바쁜 loop를 두는 조건(hog1, hog4)과 추적기 공유(shared1), strace 없는 "
        "대조(hogonly1)를 더했다.",
        "",
        f"바쁜 loop에 관한 정정: `provoke.sh`의 `with_hogs`는 run마다 loop {HOGS_PER_RUN['hog1']}개(hog4는 "
        f"{HOGS_PER_RUN['hog4']}개)를 시작하고 run 뒤에 kill하지만, kill은 감싸는 subshell에만 닿고 "
        f"`timeout {HOG_SECONDS:g} sh -c 'while :; do :; done'`은 {HOG_SECONDS:g}초를 다 돈다. 그래서 이어지는 run의 "
        "loop가 CPU 8에 쌓인다(스크립트 결함, 모든 hog run에 똑같이 해당하고 b2에서는 이전/새 binary가 같은 상태를 "
        "번갈아 받는다). 아래 표의 'CPU 8의 바쁜 loop 수'는 runs.tsv의 시작 시각으로 다시 센 값이다.",
        "",
        fmt_table(rows),
        "",
        f"false가 나온 run (모두 이전 binary):",
        "",
        fmt_table(failing) if len(failing) > 1 else "(없음)",
        "",
        f"기술 통계(판정에 쓰지 않음): b2(번갈아 실행)의 hog 조건에서 이전 {stat['b2_hog_old_false']}/"
        f"{stat['b2_hog_old_runs']}, 새 {stat['b2_hog_new_false']}/{stat['b2_hog_new_runs']}; 단측 Fisher 정확 "
        f"검정 p = {number(stat['b2_hog_fisher_one_sided_p'], '.3g')}. 새 binary 전체 {stat['new_runs']}회에서 "
        f"false가 0회이면 false 비율의 단측 95% 상한(Clopper-Pearson) = "
        f"{number(stat['new_false_rate_upper_95'], '.3%')}.",
        "",
        "## 5. 늦은 worker test (`tests/stage_batch_probe_late_workers.rs`)",
        "",
        "test binary의 global allocator가 index ≥ 1인 모든 rayon worker를 첫 할당(작업을 찾기 시작할 때, crossbeam-epoch "
        "등록)에서 300 ms 재운다. worker 0만 정상으로 시작하므로 probe pool에서 worker 0이 300 ms 동안 혼자 돈다. "
        "같은 test를 이전 probe와 새 probe에 대해 build해 번갈아 실행했다(`repeat_tests.sh`).",
        "",
        fmt_table([["probe", "실행", "통과", "실패", "비고"],
                   ["이전 (d4e2772f)", rep.get("old", {}).get("runs"), rep.get("old", {}).get("exit_zero"),
                    rep.get("old", {}).get("exit_nonzero"),
                    f"'probe observed 1' {rep.get('old', {}).get('panic_observed_1')}회"],
                   ["새 (V32)", rep.get("new", {}).get("runs"), rep.get("new", {}).get("exit_zero"),
                    rep.get("new", {}).get("exit_nonzero"),
                    f"보고서 {rep.get('new', {}).get('reports')}개: 늦은 worker {rep.get('new', {}).get('late_workers')}, "
                    f"관측 최대 {rep.get('new', {}).get('observed_max_parallel_tasks')}"]]),
        "",
        "cargo로 돌린 같은 test (이전 probe / 새 probe):",
        "",
        fmt_table([["probe", "stage_batch.rs blob", "exit", "passed", "failed", "실패 test"]] + [
            [label.split("_")[0], v["stage_batch_blob"][:8], v["exit"], v["passed"], v["failed"],
             ", ".join(v["failed_tests"]) or "-"] for label, v in t.items()]),
        "",
        "## 6. 검사 (`run_checks.sh`)",
        "",
        fmt_table(checks_rows),
        "",
        "loop checker(`check_checkpoint.py`)는 첫 불일치에서 멈춘다. 불평은 예상된 `stage_batch.rs`의 보호 blob 하나이며 "
        "(OD-L1-28에 따라 orchestrator가 재고정), `loop_checker_rest.py`는 그 한 줄만 보고로 바꾸어 나머지 검사(증거 "
        "identity, strict JSON, 변경 경로 allowlist)를 돌린다.",
        "",
        "## 7. 실행 시간 영향과 상한 (진단)",
        "",
        "정상인 경우 첫 작업은 두 번째 작업이 활성이 될 때까지만 기다린다. 늦은 worker가 있으면 그 worker가 시작할 "
        "때까지 기다리며, 이것이 예전에 실패하던 경우다. 두 번째 작업이 끝내 오지 않으면 보고서당 한 번 최대 "
        f"{code_info['rendezvous_bound_seconds']:g} s가 든다. 아래는 새 pool마다 첫 작업이 기다린 시간(첫 작업 도착부터 "
        "두 번째 작업이 동시에 활성이 될 때까지)이다. probe와 같은 구성(새 `ParallelExecution::rayon(4)`, 작업 8개, "
        "같은 rendezvous)을 무시된 test `stage_batch_probe_latency`가 재현하고 `latency.sh`가 조건별로 돌렸다. "
        "진단값이며 판정에 쓰지 않는다.",
        "",
        fmt_table([["조건", "pool 수", "두 번째 작업 없음", "최소 s", "중앙값 s", "p90 s", "p99 s", "최대 s"]] + [
            [f"{name}: {entry['header'][2:]}", entry.get("pools"), entry.get("second_task_never"),
             entry.get("min"), entry.get("median"), entry.get("p90"), entry.get("p99"), entry.get("max")]
            for name, entry in results["rendezvous_wait_diagnostic"].items()])
        if results["rendezvous_wait_diagnostic"] else "(실행 안 함)",
        "",
        "CLI run 전체의 wall 중앙값(4절 표)도 진단값이다.",
        "",
        "## 8. 남는 점",
        "",
        "- 과제 문장의 조건(strace1, strace2; 1개/2개 CPU에서 strace -f)만으로는 이 host에서 실패가 나오지 않았다. "
        "실패는 추적기와 같은 CPU에 바쁜 loop를 둔 조건과 늦은 worker test에서만 나왔다.",
        "- 2 s 상한은 늦은 worker가 2 s 넘게 시작하지 못하는 host에서는 여전히 1을 보고한다(설계상 상한).",
        "- 독립 검토와 CHECKPOINT 재고정(OD-L1-28)이 완료 조건이다.",
        "",
    ]
    return "\n".join(lines)


def main():
    batches, table, failing, per_batch = provocation()
    tests = node_tests()
    repeats = late_worker_repeats()
    check_results = checks()
    b2_hog = {}
    for binary in ("old", "new"):
        cells = [per_batch.get(("b2", binary, condition), {"runs": 0, "parallel_observed_false": 0})
                 for condition in ("hog1", "hog4")]
        b2_hog[binary] = (sum(c["parallel_observed_false"] for c in cells), sum(c["runs"] for c in cells))
    new_runs = totals(table, "new")["runs"]
    statistics_block = {
        "b2_hog_old_false": b2_hog["old"][0], "b2_hog_old_runs": b2_hog["old"][1],
        "b2_hog_new_false": b2_hog["new"][0], "b2_hog_new_runs": b2_hog["new"][1],
        "b2_hog_fisher_one_sided_p": (fisher_one_sided(*b2_hog["old"], *b2_hog["new"])
                                      if b2_hog["old"][1] and b2_hog["new"][1] else None),
        "new_runs": new_runs,
        "new_false_rate_upper_95": (1 - 0.05 ** (1 / new_runs)) if new_runs and
        totals(table, "new")["parallel_observed_false"] == 0 else None,
        "note": "descriptive; the verdicts do not use these numbers",
    }
    results = {
        "node": "V32_STAGE_BATCH_PROBE",
        "owner_decision": "OD-L1-28",
        "generated_by": "research/production_loop_20261116/stage_batch_probe/parse_results.py",
        "smoke_checksum_parallel_observed_true": SMOKE_CHECKSUM,
        "hosted_checksum_parallel_observed_false": HOSTED_FALSE_CHECKSUM,
        "code": code(),
        "provocation": {"batches": batches, "by_binary_condition": table, "failing_runs": failing,
                        "totals": {b: totals(table, b) for b in table},
                        "task_text_conditions": {b: totals(table, b, TASK_TEXT_CONDITIONS) for b in table}},
        "statistics": statistics_block,
        "node_tests": tests,
        "late_worker_test_repeats": repeats,
        "rendezvous_wait_diagnostic": latency(),
        "checks": check_results,
    }
    results["discriminators"] = discriminators(table, failing, tests, repeats, check_results)
    with open(os.path.join(HERE, "RESULTS.json"), "w", encoding="utf-8") as handle:
        json.dump(results, handle, indent=1, ensure_ascii=False, allow_nan=False)
        handle.write("\n")
    with open(os.path.join(HERE, "REPORT_KO.md"), "w", encoding="utf-8") as handle:
        handle.write(render(results))
    for d in results["discriminators"]:
        print(f"{d['verdict']:14s} {d['name']}: {d['measured']}")


if __name__ == "__main__":
    main()
