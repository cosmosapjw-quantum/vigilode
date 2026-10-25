#!/usr/bin/env python3
"""Read-only scalar export of terminal local benchmark receipts.

No solver launch, source edit, process signalling, or reference reassessment.
Use a NEW output directory outside the live run root. Raw vectors stay local.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import re

SCHEMA = "vigilode.external-benchmark-scalar-export.v1"
RECEIPT_NAMES = ("RECEIPT.json", "COMPARISON_RECEIPT.json")
TERMINAL = {"COMPLETED", "TIMEOUT", "FAILED", "PROCESS_FAILED", "SPAWN_FAILED",
            "PARSE_FAILED", "INTERRUPTED", "CONTRACT_FAILED", "RESOURCE_LIMIT",
            "SPAWN_FAILURE", "PROCESS_FAILURE", "MALFORMED_RESULT"}
LIMIT = 64 * 1024 * 1024


def digest_bytes(data):
    return hashlib.sha256(data).hexdigest()


def reject_constant(value):
    raise ValueError("nonfinite JSON token: " + value)


def loads(data):
    return json.loads(data, parse_constant=reject_constant)


def finite_number(value):
    if type(value) not in (int, float):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def bounded_read(path):
    before = path.stat()
    if before.st_size > LIMIT:
        raise ValueError("file exceeds 64 MiB export read limit")
    data = path.read_bytes()
    after = path.stat()
    if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
        raise ValueError("file changed while reading")
    return data


def scalars(value, prefix=""):
    """Flatten dictionaries, preserving scalar fields and omitting all arrays."""
    out = {}
    if not isinstance(value, dict):
        return out
    for key, item in value.items():
        name = prefix + str(key)
        if isinstance(item, dict):
            out.update(scalars(item, name + "."))
        elif item is None or isinstance(item, (str, bool, int, float)):
            if isinstance(item, float) and not math.isfinite(item):
                raise ValueError("nonfinite scalar: " + name)
            out[name] = item
    return out


def inside(path, root):
    resolved = path.resolve()
    if not resolved.is_relative_to(root):
        raise ValueError("symlink resolves outside run root")
    return resolved


def export_receipt(path, root):
    row = {"receipt_path": str(path.relative_to(root)), "receipt_name": path.name,
           "evidence_status": "UNREAD", "issues": [], "timing_reanalysis_candidate": False}
    try:
        receipt_bytes = bounded_read(inside(path, root))
        row["receipt_sha256"] = digest_bytes(receipt_bytes)
        outer = loads(receipt_bytes)
        if not isinstance(outer, dict):
            raise ValueError("receipt is not an object")
        raw = outer.get("raw_receipt", outer)
        if not isinstance(raw, dict):
            raise ValueError("raw_receipt is not an object")
        execution = raw.get("execution_status")
        row["execution_status"] = execution
        # A receipt is terminal only when it has a recognised terminal label
        # and a finish timestamp or exit code. START snapshots alone never pass.
        if execution not in TERMINAL or not (raw.get("finished_utc") or "exit_code" in raw):
            row["evidence_status"] = "NONTERMINAL_OR_UNSUPPORTED_RECEIPT"
            return row
        row["receipt_scalars"] = scalars({k: v for k, v in raw.items()
            if k not in ("result", "task", "source", "raw_receipt")})
        row["source_scalars"] = scalars(raw.get("source", {}))
        row["task_scalars"] = scalars(raw.get("task", {}))
        row["comparison_scalars"] = scalars({k: v for k, v in outer.items()
            if k != "raw_receipt"}) if outer is not raw else {}
        row["task_id"] = raw.get("task", {}).get("id")
        row["stream_sha256"] = {}
        stdout = None
        row["resource_scalars"] = {}
        for label, filename in (("stdout", "stdout.jsonl"), ("stderr", "stderr.log"),
                                ("resource", "resource.txt")):
            expected = raw.get(label + "_sha256")
            stream = path.parent / filename
            if expected is None:
                if label in ("stdout", "stderr"):
                    row["issues"].append("UNBOUND_" + label.upper())
                continue
            if not stream.is_file():
                row["issues"].append("MISSING_" + label.upper())
                continue
            data = bounded_read(inside(stream, root))
            actual = digest_bytes(data)
            row["stream_sha256"][label] = actual
            if expected != actual:
                row["issues"].append("HASH_MISMATCH_" + label.upper())
            if label == "stdout":
                stdout = data
            if label == "resource":
                for line in data.decode("utf-8", errors="replace").splitlines():
                    name, separator, value = line.strip().rpartition(":")
                    if separator and name in ("User time (seconds)", "System time (seconds)",
                            "Maximum resident set size (kbytes)", "Voluntary context switches",
                            "Involuntary context switches", "Major (requiring I/O) page faults",
                            "Minor (reclaiming a frame) page faults"):
                        try:
                            number = float(value.strip())
                            if finite_number(number) and number >= 0:
                                row["resource_scalars"][name] = number
                        except ValueError:
                            pass
        parsed_results = []
        parsed_headers = []
        if stdout is not None:
            for line_number, line in enumerate(stdout.splitlines(), 1):
                if not line.strip():
                    continue
                try:
                    value = loads(line)
                except (ValueError, UnicodeDecodeError):
                    row["issues"].append("INVALID_STDOUT_JSON_LINE_" + str(line_number))
                    continue
                if isinstance(value, dict) and value.get("kind") == "result":
                    parsed_results.append(value)
                if isinstance(value, dict) and value.get("kind") == "header":
                    parsed_headers.append(value)
        row["header_scalars"] = scalars(parsed_headers[0]) if len(parsed_headers) == 1 else {}
        if len(parsed_headers) > 1:
            row["issues"].append("MULTIPLE_HEADERS")
        stored = raw.get("result")
        parsed = parsed_results[0] if len(parsed_results) == 1 else None
        if len(parsed_results) > 1:
            row["issues"].append("MULTIPLE_RESULTS")
        if stored != parsed:
            row["issues"].append("STORED_RAW_RESULT_MISMATCH")
        if execution == "COMPLETED" and parsed is None:
            row["issues"].append("COMPLETED_WITHOUT_RESULT")
        result = parsed if isinstance(parsed, dict) else {}
        row["result_scalars"] = scalars(result)
        row["omitted_array_fields"] = [k for k, v in result.items() if isinstance(v, list)]
        task = raw.get("task", {})
        for key in ("method", "parallel_layer", "threads"):
            if key in task and result and result.get(key) != task[key]:
                row["issues"].append("CONTRACT_MISMATCH_" + key.upper())
        if "case" in task and result and result.get("case_id", result.get("case")) != task["case"]:
            row["issues"].append("CONTRACT_MISMATCH_CASE")
        if outer is not raw:
            for key in ("method", "case", "size"):
                if key in outer and result:
                    actual = result.get("case_id", result.get("case")) if key == "case" else result.get(key)
                    if actual is not None and actual != outer[key]:
                        row["issues"].append("WRAPPER_MISMATCH_" + key.upper())
        for name in ("elapsed_seconds", "final_time"):
            if result.get(name) is not None and not finite_number(result[name]):
                row["issues"].append("INVALID_" + name.upper())
        if result.get("elapsed_seconds") is not None and finite_number(result["elapsed_seconds"]) and result["elapsed_seconds"] <= 0:
            row["issues"].append("NONPOSITIVE_ELAPSED_SECONDS")
        if result.get("solver_success") is False and (result.get("scientific_status", result.get("status")) in ("PASS", "PASS_SCOPED")):
            row["issues"].append("SOLVER_SUCCESS_STATUS_CONFLICT")
        row["evidence_status"] = "RAW_VERIFIED" if not row["issues"] else "EVIDENCE_ISSUES"
        assessment = outer.get("assessment", {}) if outer is not raw else {}
        science = assessment.get("status", result.get("scientific_status", result.get("status")))
        row["original_scientific_status"] = science
        error = assessment.get("actual_error", result.get("actual_error"))
        cap = assessment.get("error_limit", result.get("error_limit"))
        uncertainty = assessment.get("reference_uncertainty_bound", result.get("reference_absolute_bound", 0))
        elapsed = result.get("elapsed_seconds")
        source = raw.get("source", {})
        # This flags candidates for a later matched comparison. It is not a new
        # accuracy claim or proof of equivalence of methods, references or hosts.
        row["timing_reanalysis_candidate"] = bool(not row["issues"] and execution == "COMPLETED"
            and raw.get("exit_code") == 0 and science in ("PASS", "PASS_SCOPED")
            and result.get("solver_success") is not False
            and not task.get("instrumented") and not result.get("instrumented")
            and task.get("purpose") != "accuracy-probe" and not outer.get("warmup")
            and not source.get("exploratory_dirty_run")
            and finite_number(elapsed) and elapsed > 0
            and finite_number(error) and error >= 0 and finite_number(cap) and cap > 0
            and finite_number(uncertainty) and uncertainty >= 0 and error + uncertainty <= cap)
        # Detect a receipt rewritten while its stream files were being read.
        if bounded_read(inside(path, root)) != receipt_bytes:
            row["issues"].append("RECEIPT_CHANGED_DURING_EXPORT")
            row["evidence_status"] = "EVIDENCE_ISSUES"
            row["timing_reanalysis_candidate"] = False
    except (OSError, ValueError, TypeError, AttributeError, OverflowError) as error:
        row["evidence_status"] = "READ_OR_FORMAT_ERROR"
        row["timing_reanalysis_candidate"] = False
        row["issues"].append(type(error).__name__ + ": " + str(error))
    return row


def export(root, output, task_index=None):
    root = Path(root).resolve(strict=True)
    output = Path(output).resolve()
    if not root.is_dir() or output.is_relative_to(root):
        raise ValueError("run root must be a directory and output must be outside it")
    if output.exists():
        raise FileExistsError("output exists; choose a new directory")
    paths = sorted({p for name in RECEIPT_NAMES for p in root.rglob(name)})
    rows = [export_receipt(path, root) for path in paths]
    # Wrappers and their underlying receipts are both retained, with a common
    # evidence key. Consumers must not count them as independent repetitions.
    for row in rows:
        row["raw_evidence_key"] = str(Path(row["receipt_path"]).parent) + ":" + row.get("stream_sha256", {}).get("stdout", "MISSING")
    incomplete = []
    for start in sorted(root.rglob("START.json")):
        if not any((start.parent / name).exists() for name in RECEIPT_NAMES):
            incomplete.append({"start_path": str(start.relative_to(root)),
                "status": "INCOMPLETE_ACTIVE_OR_INTERRUPTED", "action": "not inspected or interrupted"})
    index_summary = None
    if task_index is not None:
        index_bytes = bounded_read(Path(task_index))
        index = loads(index_bytes)
        indexed = index.get("rows")
        if not isinstance(indexed, list):
            raise ValueError("task index must contain rows")
        hashes = {row["receipt_sha256"] for row in rows if row.get("receipt_sha256")}
        identities = {(row["task_id"], row["receipt_sha256"]) for row in rows
                      if row.get("task_id") and row.get("receipt_sha256")}
        available = {row["task_id"] for row in rows if row.get("task_id")}
        index_summary = {"sha256": digest_bytes(index_bytes), "tasks": len(indexed), "rows": []}
        for old in indexed:
            if (not isinstance(old, dict) or not isinstance(old.get("task_id"), str)
                    or not old["task_id"].strip() or not isinstance(old.get("receipt_sha256"), str)
                    or re.fullmatch(r"[0-9a-f]{64}", old["receipt_sha256"]) is None):
                raise ValueError("task index row requires nonempty task_id and SHA256")
            if (old["task_id"], old["receipt_sha256"]) in identities:
                status = "EXACT_RECEIPT_BYTES_FOUND"
            elif old["receipt_sha256"] in hashes:
                status = "RECEIPT_TASK_ID_MISMATCH"
            elif old.get("task_id") in available:
                status = "TASK_FOUND_DIFFERENT_RECEIPT_BYTES"
            else:
                status = "RAW_RECEIPT_NOT_FOUND"
            index_summary["rows"].append({"task_id": old.get("task_id"), "receipt_sha256": old.get("receipt_sha256"), "status": status})
        index_summary["counts"] = dict(Counter(x["status"] for x in index_summary["rows"]))
    summary = {"schema": SCHEMA, "run_root": str(root),
        "receipt_files": len(rows), "raw_evidence_keys": len({r["raw_evidence_key"] for r in rows}),
        "evidence_status_counts": dict(Counter(r["evidence_status"] for r in rows)),
        "original_scientific_status_counts": dict(Counter(str(r.get("original_scientific_status")) for r in rows)),
        "incomplete_attempts": incomplete,
        "policy": "Terminal scalar export only; original scientific statuses retained; no speed ranking or reference reassessment; final vectors omitted; wrappers share raw_evidence_key.",
        "task_index": index_summary, "exporter_sha256": digest_bytes(Path(__file__).read_bytes())}
    output.mkdir(parents=True, exist_ok=False)
    with (output / "MEASUREMENTS.jsonl").open("x", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, sort_keys=True, allow_nan=False) + "\n")
    summary["measurements_sha256"] = digest_bytes((output / "MEASUREMENTS.jsonl").read_bytes())
    (output / "EXPORT_SUMMARY.json").write_text(json.dumps(summary, indent=2, sort_keys=True, allow_nan=False) + "\n")
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--task-index", type=Path)
    args = parser.parse_args()
    result = export(args.run_root, args.output, args.task_index)
    print(json.dumps({k: result[k] for k in ("receipt_files", "evidence_status_counts", "measurements_sha256")}, sort_keys=True))


if __name__ == "__main__":
    main()
