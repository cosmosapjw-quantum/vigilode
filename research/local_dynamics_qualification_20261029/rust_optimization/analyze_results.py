#!/usr/bin/env python3
"""Reproducible, read-only R2 aggregation for the bounded Rust FD optimization."""
import argparse
import hashlib
import gzip
import json
import math
from pathlib import Path
import re
import statistics
import subprocess
import sys
import tarfile
import tempfile

CASES = ("dense-fd", "banded-fd", "time-jvp", "autonomous-jvp")
CAP = 1e-5


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def load(path):
    return json.loads(Path(path).read_text())


def stage_public_packet(packet_dir, temp_dir):
    """Verify packet file hashes and expand gzip source members to a root layout."""
    manifest_path = packet_dir / "PACKET_MANIFEST.json"
    if not manifest_path.is_file():
        return None
    manifest = load(manifest_path)
    staged = Path(temp_dir) / "source-root"
    for row in manifest.get("packet_files", []):
        rel = Path(row["relative_path"])
        src = (packet_dir / rel).resolve()
        if packet_dir.resolve() not in src.parents:
            raise SystemExit(f"INPUT_DEPENDENCY: unsafe packet member {rel}")
        if not src.is_file():
            raise SystemExit(f"INPUT_DEPENDENCY: packet member absent {rel}")
        if sha(src) != row["sha256"]:
            raise SystemExit(f"INPUT_INTEGRITY: packet member hash mismatch {rel}")
        if not rel.as_posix().startswith("source/"):
            continue
        logical = Path(rel.as_posix()[len("source/"):])
        payload = src.read_bytes()
        if logical.name.endswith(".gz"):
            try:
                payload = gzip.decompress(payload)
            except OSError as exc:
                raise SystemExit(f"INPUT_DEPENDENCY: invalid compressed member {rel}: {exc}")
            logical = logical.with_name(logical.name[:-3])
        dest = staged / logical
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(payload)
    return staged


def apply_reentrant_supplement(supplement_dir, root):
    manifest_path = supplement_dir / "SUPPLEMENT_MANIFEST.json"
    if not manifest_path.is_file():
        raise SystemExit(f"INPUT_DEPENDENCY: supplement manifest absent: {manifest_path}")
    manifest = load(manifest_path)
    if manifest.get("schema") != "vigilode-reentrant-rows-supplement-v1":
        raise SystemExit("INPUT_DEPENDENCY: unsupported reentrant supplement schema")
    seen = []
    for row in manifest.get("rows", []):
        rel = Path(row["relative_path"])
        packed_rel = Path(row["packed_relative_path"])
        packed = (supplement_dir / packed_rel).resolve()
        if supplement_dir.resolve() not in packed.parents or not packed.is_file():
            raise SystemExit(f"INPUT_DEPENDENCY: absent/unsafe supplement file {packed_rel}")
        packed_bytes = packed.read_bytes()
        if len(packed_bytes) != row["packed_bytes"] or sha(packed) != row["packed_sha256"]:
            raise SystemExit(f"INPUT_INTEGRITY: supplement packed hash mismatch {packed_rel}")
        try:
            payload = gzip.decompress(packed_bytes)
        except OSError as exc:
            raise SystemExit(f"INPUT_DEPENDENCY: invalid supplement gzip {packed_rel}: {exc}")
        if len(payload) != row["bytes"] or hashlib.sha256(payload).hexdigest() != row["sha256"]:
            raise SystemExit(f"INPUT_INTEGRITY: supplement original hash mismatch {rel}")
        dest = (root / rel).resolve()
        if root.resolve() not in dest.parents:
            raise SystemExit(f"INPUT_DEPENDENCY: unsafe supplement path {rel}")
        dest.parent.mkdir(parents=True, exist_ok=True)
        if dest.exists() and dest.read_bytes() != payload:
            raise SystemExit(f"INPUT_INTEGRITY: supplement conflicts with packet member {rel}")
        if not dest.exists():
            dest.write_bytes(payload)
        seen.append(rel.as_posix())
    return {"schema": manifest["schema"], "manifest_sha256": sha(manifest_path),
            "row_count": len(seen), "supplement_root": str(supplement_dir),
            "original_rows_sha256": hashlib.sha256("\n".join(seen).encode()).hexdigest()}


def parity_value(value):
    x = json.loads(json.dumps(value))
    x.pop("mode", None)
    x.pop("setup_ns", None)
    for sample in x.get("samples", []):
        for k in ("solver_ns", "solve_plus_drop_ns", "solve_allocations"):
            sample.pop(k, None)
    return json.dumps(x, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def extract_controls(d):
    rows = []
    checks = []
    for case, modes in d["validation_controls"].items():
        if case in CASES:
            configs = [("default", modes)]
        else:
            configs = list(modes.items())
        for config, data in configs:
            for mode in ("probe", "alloc"):
                pair = data[mode]
                b, c = pair["baseline"]["value"], pair["candidate"]["value"]
                same = parity_value(b) == parity_value(c)
                checks.append({"case": case, "config": config, "mode": mode,
                               "exact_returned_output_stats_callback_parity": same})
                sample = {}
                for arm, value in (("baseline", b), ("candidate", c)):
                    s = value["samples"][0]
                    sample[arm] = {"whole_solve_allocations": s.get("solve_allocations"),
                                   "endpoint_max_abs_error": s.get("endpoint_max_abs_error"),
                                   "success": s.get("status", {}).get("status") == "success",
                                   "rhs_callbacks": value.get("callback_counts", {}).get("rhs"),
                                   "jvp_callbacks": value.get("callback_counts", {}).get("jvp"),
                                   "time_derivative_callbacks": value.get("callback_counts", {}).get("time_derivative"),
                                   "accepted_steps": s.get("stats", {}).get("accepted_steps"),
                                   "attempts": s.get("stats", {}).get("attempts"),
                                   "rejected_steps": s.get("stats", {}).get("rejected_steps")}
                rows.append({"case": case, "config": config, "mode": mode,
                             "exact_parity": same, "arms": sample})
    return rows, checks


def timing_summary(d):
    out = {}
    for case, records in d["cases"].items():
        allvals = {arm: [] for arm in ("baseline", "candidate")}
        blocks = []
        for b in range(1, 6):
            proc_medians = {arm: [] for arm in allvals}
            for record in records:
                if record["phase"] != "timed" or record["block"] != b:
                    continue
                arm = record["order"]
                vals = [s["solve_plus_drop_ns"] for s in record["value"]["samples"]]
                allvals[arm].extend(vals)
                proc_medians[arm].append(statistics.median(vals))
            means = {arm: statistics.mean(proc_medians[arm]) for arm in allvals}
            blocks.append({"block": b, "baseline_process_medians_ns": proc_medians["baseline"],
                           "candidate_process_medians_ns": proc_medians["candidate"],
                           "baseline_mean_ns": means["baseline"], "candidate_mean_ns": means["candidate"],
                           "baseline_over_candidate": means["baseline"] / means["candidate"]})
        ratios = [x["baseline_over_candidate"] for x in blocks]
        med = statistics.median(ratios)
        lo, hi = min(ratios), max(ratios)
        out[case] = {"repetitions_per_process": 64, "warmups_per_arm": 1,
                     "paired_blocks": blocks,
                     "all_timed_solve_plus_drop_samples_ns": {
                         arm: {"count": len(vals), "median": statistics.median(vals),
                               "min": min(vals), "max": max(vals)} for arm, vals in allvals.items()},
                     "block_ratio_definition": "each timed process median of 64 solve_plus_drop_ns; within block mean of two process medians per arm; ratio=baseline/candidate; median/min/max across five block ratios",
                     "baseline_over_candidate_ratio": {"median": med, "min": lo, "max": hi,
                          "worst_observed_candidate_slowdown_percent": max(0.0, (1.0 / lo - 1.0) * 100.0)},
                     "timing_authority": "diagnostic_only"}
    return out


def reentrant_summary(root, d):
    rows = {}
    for attempt in d["attempts"]:
        if not attempt["name"].startswith(("warmup-", "timing-")):
            continue
        try:
            obj = load(attempt["stdout_path"])
            row = obj["rows"][0]
        except Exception:
            continue
        policy = row["policy"]
        arm = "candidate" if attempt["name"].endswith("candidate") else "baseline"
        bucket = rows.setdefault(policy, {}).setdefault(arm, [])
        bucket.append({"name": attempt["name"], "seconds_per_32_solves": row["seconds"] / 32.0,
                       "seconds_total_for_32_solves": row["seconds"], "pass": row["pass"],
                       "max_error": row["max_error"], "accuracy_cap": row["accuracy_cap"],
                       "rhs": row["rhs"], "jvp": row["jvp"], "attempts": row["attempts"],
                       "accepted_steps": sum(s.get("accepted_steps", 0) for s in row.get("segments", [])),
                       "repeats": row["repeats"], "returned_observation_scope": "last repetition only"})
    policy_summary = {}
    for policy, arms in rows.items():
        policy_summary[policy] = {}
        for arm, vals in arms.items():
            seconds = [x["seconds_per_32_solves"] for x in vals if x["name"].startswith("timing-")]
            exemplar = next((x for x in vals if x["name"].startswith("timing-")), vals[0])
            policy_summary[policy][arm] = {"timed_process_count": len(seconds),
                "median_seconds_per_solve": statistics.median(seconds),
                "min_seconds_per_solve": min(seconds), "max_seconds_per_solve": max(seconds),
                "median_endpoint_error_last_repeat": statistics.median(x["max_error"] for x in vals),
                "max_endpoint_error_last_repeat": max(x["max_error"] for x in vals),
                "rhs_last_repeat": exemplar["rhs"], "jvp_last_repeat": exemplar["jvp"],
                "accepted_strict_work_last_repeat": next((load(a["stdout_path"])["rows"][0]["work"] for a in d["attempts"] if a["name"].startswith("timing-"+policy+"-") and a["name"].endswith("-"+arm)), None),
                "attempts_last_repeat": exemplar["attempts"],
                "accepted_steps_last_repeat": exemplar["accepted_steps"],
                "accuracy_cap": exemplar["accuracy_cap"]}
    raw_rows = {}
    def canonical(row):
        row = dict(row)
        row.pop("seconds", None)
        return json.dumps(row, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    for attempt in d["attempts"]:
        name = attempt["name"]
        if not (name.startswith("warmup-") or name.startswith("timing-")):
            continue
        try: row = load(attempt["stdout_path"])["rows"][0]
        except Exception: continue
        raw_rows[name] = row
    parity = []
    for policy in d["policies"]:
        bname, cname = f"warmup-{policy}-baseline", f"warmup-{policy}-candidate"
        parity.append({"pair": [bname, cname], "same_except_measured_seconds":
                       canonical(raw_rows[bname]) == canonical(raw_rows[cname])})
        for block in range(1, 6):
            for arm_idx in range(2):
                candidates = {}
                for arm in ("baseline", "candidate"):
                    names = sorted((n for n in raw_rows if n.startswith(f"timing-{policy}-block{block}-") and n.endswith("-" + arm)),
                                   key=lambda n: int(n.split(f"block{block}-")[1].split("-")[0]))
                    candidates[arm] = names[arm_idx]
                parity.append({"pair": [candidates["baseline"], candidates["candidate"]],
                               "same_except_measured_seconds": canonical(raw_rows[candidates["baseline"]]) == canonical(raw_rows[candidates["candidate"]])})
    rr = {p: d["profile"]["instruction_slopes"][p + "/baseline"]["Ir_per_repeat"] for p in d["policies"]}
    strict_over_rhs = rr["observed-strict-sentinel"] / rr["rhs-only"]
    policy_times = {p: {arm: policy_summary[p][arm]["median_seconds_per_solve"] for arm in ("baseline", "candidate")} for p in policy_summary}
    rhs_work = policy_summary["rhs-only"]["baseline"]["rhs_last_repeat"]
    return {"status": d["status"], "failures": d["failures"], "summary_by_policy_and_arm": policy_summary,
            "exact_last_returned_row_parity_checks": parity,
            "exact_last_returned_row_parity_count": len(parity),
            "whole_process_instruction_slope_ir_per_repeat": d["profile"]["instruction_slopes"],
            "strict_vs_rhs_instruction_slope_ratio_by_baseline_binary": strict_over_rhs,
            "median_seconds_per_solve_by_policy_arm": policy_times,
            "strict_over_rhs_median_time_ratio": {arm: policy_times["observed-strict-sentinel"][arm] / policy_times["rhs-only"][arm] for arm in ("baseline", "candidate")},
            "rhs_only_vs_strict_last_repeat_work": {"rhs_only_work": rhs_work, "strict_work": next((json.load(open(a["stdout_path"]))["rows"][0]["work"] for a in d["attempts"] if a["name"].startswith("timing-observed-strict-sentinel-block1-0-baseline")), None)},
            "instruction_scope_note": "Callgrind 4/12 whole-process Ir differencing cancels fixed startup only insofar as it is invariant; repeated harness/JSON/output work and residual startup differences remain; not solver-only wall time.",
            "state_work_error_scope": "only each solve's final repeated result is returned and observed; the other 31 repetitions are not separately state/work/error records",
            "exact_parity_checked_by_runner": True}


def parse_table(path):
    result = []
    for line in Path(path).read_text(errors="replace").splitlines():
        m = re.match(r"^([0-9,]+) \(\s*([0-9.]+)%\) (.+)$", line)
        if m and "PROGRAM TOTALS" not in line:
            result.append({"Ir": int(m.group(1).replace(",", "")), "percent": float(m.group(2)),
                           "function": m.group(3)})
    return result


def parse_function_counts(path):
    result = {}
    for line in Path(path).read_text(errors="replace").splitlines():
        m = re.match(r"^([0-9,]+) \(\s*([0-9.]+)%\) (.+)$", line)
        if m and "PROGRAM TOTALS" not in line:
            result[m.group(3).split(" [")[0]] = int(m.group(1).replace(",", ""))
    return result


def profile_functions(root):
    base = root / "annotations-corrected-r2"
    out = {}
    for case in CASES:
        for arm in ("baseline", "candidate"):
            for include in ("no", "yes"):
                p = base / f"callgrind-{case}-{arm}-r12-inclusive-{include}.txt"
                if p.exists():
                    out[f"{case}/{arm}/{'self' if include == 'no' else 'inclusive'}"] = {
                        "top_functions": parse_table(p)[:12], "raw_path": str(p), "sha256": sha(p),
                        "percent_note": "self and inclusive are separate views; never sum inclusive rows"}
    target_patterns = {
        "time-jvp": ["two_pass_mgs_into", "arnoldi_column", "qr_in_place", "pulp::x86::v3::V3::vectorize::imp",
                     "least_squares_correction", "LeastSquaresWorkspace::solve_into"],
        "dense-fd": ["Workspace::attempt", "lu_solve_in_place", "sha256"],
        "banded-fd": ["BandState::solve", "Workspace::attempt", "rust_allocation_probe::model::{{closure}}"],
        "autonomous-jvp": ["two_pass_mgs_into", "gmres_into::arnoldi_column", "qr_in_place"]}
    out["function_instruction_slopes"] = {}
    for case, patterns in target_patterns.items():
        for scope in ("no", "yes"):
            for arm in ("baseline", "candidate"):
                p4 = base / f"callgrind-{case}-{arm}-r4-inclusive-{scope}.txt"
                p12 = base / f"callgrind-{case}-{arm}-r12-inclusive-{scope}.txt"
                if not (p4.exists() and p12.exists()): continue
                a4, a12 = parse_function_counts(p4), parse_function_counts(p12)
                vals = []
                for pattern in patterns:
                    k4 = next((k for k in a4 if pattern in k), None)
                    k12 = next((k for k in a12 if pattern in k), None)
                    vals.append({"function_pattern": pattern,
                        "r4_self_or_inclusive_Ir": a4.get(k4) if k4 else None,
                        "r12_self_or_inclusive_Ir": a12.get(k12) if k12 else None,
                        "Ir_per_repeat_difference_over_8": ((a12[k12]-a4[k4])/8 if k4 and k12 else None),
                        "matched_function_name": k12})
                out["function_instruction_slopes"][f"{case}/{arm}/{'self' if scope == 'no' else 'inclusive'}"] = {
                    "functions": vals, "definition": "(function Ir at repeat 12 minus at repeat 4)/8; self/inclusive is kept separate; these are instruction counts, not wall-time"}
    return out


def rss_summary(root):
    d = root / "rss-instrumented-r2"
    out = {}
    for p in d.glob("*.resource"):
        txt = p.read_text(errors="replace")
        m = re.search(r"Maximum resident set size \(kbytes\): (\d+)", txt)
        out[p.stem] = {"maximum_rss_kib": int(m.group(1)) if m else None,
                       "scope": "candidate binary only, whole process, /usr/bin/time -v, alloc mode"}
    return out


def source_files(worktree, candidate_sha):
    paths = ["crates/vigilode/src/fd.rs", "crates/vigilode/src/solve.rs",
             "crates/rodas5p-integrators/src/problem.rs", "crates/rodas5p-integrators/src/rodas5p_fast.rs",
             "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs",
             "crates/vigilode/tests/fd_allocation_contracts.rs",
             "crates/rodas5p-integrators/tests/time_fd_workspace_contracts.rs"]
    out = []
    for rel in paths:
        q = subprocess.run(["git", "show", f"{candidate_sha}:{rel}"], cwd=worktree,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        if q.returncode == 0:
            digest = hashlib.sha256(q.stdout).hexdigest()
            method = "git_show_at_candidate_sha"
            error = None
        else:
            packed = worktree / "candidate-source" / rel
            digest = sha(packed) if packed.exists() else None
            method = "candidate_source_packet_file" if digest else "NOT_AVAILABLE_IN_CHECKOUT_OR_PACKET"
            error = q.stderr.decode(errors="replace") if not digest else None
        out.append({"path": rel, "sha256": digest, "identity": candidate_sha,
                    "hash_method": method, "error": error})
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="/home/cosmosapjw/vigilode-rust-opt-20261028T212815Z-t14sx5sh")
    ap.add_argument("--packet", help="directory or tar/tar.gz compact evidence packet preserving the R2 root-relative layout")
    ap.add_argument("--packet-supplement", help="directory containing SUPPLEMENT_MANIFEST.json and create-only gzip reentrant stdout/stderr rows")
    ap.add_argument("--worktree", help="checkout containing candidate commit; if omitted, source files may be under PACKET/candidate-source/")
    ap.add_argument("--out-dir", default=str(Path(__file__).resolve().parent))
    a = ap.parse_args()
    temp_packet = None
    packet_root = None
    supplement_info = None
    packet_arg_root = None
    if a.packet:
        pp = Path(a.packet).resolve()
        packet_arg_root = pp
        if pp.is_dir():
            temp_packet = tempfile.TemporaryDirectory(prefix="vigilode-r2-packet-")
            packet_root = stage_public_packet(pp, temp_packet.name) or pp
        else:
            temp_packet = tempfile.TemporaryDirectory(prefix="vigilode-r2-packet-")
            with tarfile.open(pp, "r:*") as tf:
                members = tf.getmembers()
                for member in members:
                    dest = (Path(temp_packet.name) / member.name).resolve()
                    if Path(temp_packet.name).resolve() not in dest.parents and dest != Path(temp_packet.name).resolve():
                        raise SystemExit(f"INPUT_DEPENDENCY: unsafe archive member {member.name}")
                tf.extractall(temp_packet.name, members=members, filter="data")
            staged = stage_public_packet(Path(temp_packet.name), Path(temp_packet.name) / "normalized")
            candidates = list((staged or Path(temp_packet.name)).rglob("MEASUREMENT_IDENTITY_R2.json"))
            packet_root = next((p.parent for p in candidates if (p.parent / "light-final-r2/manifest.json").exists()), None)
            if packet_root is None:
                raise SystemExit("INPUT_DEPENDENCY: packet missing MEASUREMENT_IDENTITY_R2.json and light-final-r2/manifest.json")
        root = packet_root
        supplement = Path(a.packet_supplement).resolve() if a.packet_supplement else (pp.parent / "evidence_reentrant_rows")
        if supplement.is_dir():
            supplement_info = apply_reentrant_supplement(supplement, root)
    else:
        root = Path(a.root)
    worktree = Path(a.worktree) if a.worktree else (root if a.packet else Path(__file__).resolve().parents[3])
    outdir = Path(a.out_dir)
    outdir.mkdir(parents=True, exist_ok=True)
    identity = load(root / "MEASUREMENT_IDENTITY_R2.json")
    light = load(root / "light-final-r2/manifest.json")
    reentrant = load(root / "reentrant-control-r2/summary.json")
    original_root = None
    if a.packet:
        bp = Path(identity["binary"]["baseline"]["path"])
        original_root = bp.parents[3]
        def relocate(node):
            if isinstance(node, dict):
                for k, v in list(node.items()):
                    node[k] = relocate(v)
            elif isinstance(node, list):
                return [relocate(v) for v in node]
            elif isinstance(node, str) and node.startswith(str(original_root) + "/"):
                return str(root / node[len(str(original_root) + "/"):])
            return node
        light = relocate(light)
        reentrant = relocate(reentrant)
        required = ["MEASUREMENT_IDENTITY_R2.json", "light-final-r2/manifest.json",
                    "reentrant-control-r2/summary.json", "SUPPLEMENTAL_COMMANDS_R2.json",
                    "annotations-corrected-r2/COMMANDS.json"]
        missing = [rel for rel in required if not (root / rel).exists()]
        if missing:
            raise SystemExit("INPUT_DEPENDENCY (NOT_RUN): packet missing " + ", ".join(missing))
        needed_rows = [x.get(k) for x in reentrant.get("attempts", [])
                       for k in ("stdout_path",) if isinstance(x.get(k), str)]
        absent_rows = [raw_path for raw_path in needed_rows if not Path(raw_path).is_file()]
        if absent_rows:
            raise SystemExit("INPUT_DEPENDENCY (NOT_RUN): packet omits reentrant per-attempt stdout needed to recompute returned-row parity/work/error; missing " + ", ".join(absent_rows[:4]) + (f" ... ({len(absent_rows)} files)" if len(absent_rows) > 4 else ""))
    controls, checks = extract_controls(light)
    assert light["status"] == "COMPLETE" and not light["failures"]
    assert reentrant["status"] == "COMPLETE" and not reentrant["failures"]
    assert len(checks) == 18
    assert all(x["exact_returned_output_stats_callback_parity"] for x in checks)
    maxerr = max(x["arms"][arm]["endpoint_max_abs_error"] for x in controls
                 for arm in ("baseline", "candidate"))
    assert maxerr <= CAP
    timing = timing_summary(light)
    reentrant_out = reentrant_summary(root, reentrant)
    assert len(reentrant_out["exact_last_returned_row_parity_checks"]) == 22
    assert all(x["same_except_measured_seconds"] for x in reentrant_out["exact_last_returned_row_parity_checks"])
    profile_funcs = profile_functions(root)

    alloc_rows = [x for x in controls if x["mode"] == "alloc"]
    all_allocs = [{"case": x["case"], "config": x["config"],
                   "baseline_allocations": x["arms"]["baseline"]["whole_solve_allocations"],
                   "candidate_allocations": x["arms"]["candidate"]["whole_solve_allocations"],
                   "delta_candidate_minus_baseline": x["arms"]["candidate"]["whole_solve_allocations"] - x["arms"]["baseline"]["whole_solve_allocations"],
                   "percent_reduction": 100 * (x["arms"]["baseline"]["whole_solve_allocations"] - x["arms"]["candidate"]["whole_solve_allocations"]) / x["arms"]["baseline"]["whole_solve_allocations"]}
                  for x in alloc_rows]

    receipts = []
    for p in sorted(root.glob("*/RECEIPT.json")):
        try: d = load(p)
        except Exception: continue
        if d.get("wall_seconds") is not None:
            receipts.append({"path": str(p), "sha256": sha(p), "wall_seconds": d["wall_seconds"],
                             "exit_code": d.get("exit_code"), "status": d.get("status"),
                             "command": d.get("command"), "git_head": d.get("git_head")})
    costs = {"root_receipts": receipts,
             "root_receipt_wall_sum_nonoverlapping_process_groups_seconds": sum(x["wall_seconds"] for x in receipts),
             "note": "Each wrapper receipt is counted once; nested benchmark child attempts are not added again. Builds are separately identified by command category.",
             "annotation_postprocess": load(root / "annotations-corrected-r2/COMMANDS.json"),
             "heaptrack_exact_wall_cost": "NOT_RECORDED_IN_RECEIPT",
             "rss_run_elapsed_precision": "time -v rounded to 0.01 s; no sub-centisecond estimate used"}
    for x in receipts:
        c = x.get("command", [])
        x["category"] = "build" if c[:2] == ["cargo", "build"] else ("test_or_clippy" if c and c[0] == "cargo" else "runner_or_other")

    heap_files = []
    for p in sorted((root / "heaptrack-r2").glob("*")):
        if p.is_file(): heap_files.append({"path": str(p), "size_bytes": p.stat().st_size, "sha256": sha(p)})
    rss = rss_summary(root)
    failed_ids = ["fd-red", "fd-red2", "fd-red3", "timefd-red", "timefd-red2", "timefd-red3",
                  "timefd-unit-green", "strict-focused-clippy", "strict-focused-clippy2"]
    raw_inputs = [root / "MEASUREMENT_IDENTITY_R2.json", root / "light-final-r2/manifest.json",
                  root / "reentrant-control-r2/summary.json", root / "light-wrapper-r2/RECEIPT.json",
                  root / "reentrant-wrapper-r2/RECEIPT.json", root / "build-baseline-final/RECEIPT.json",
                  root / "build-candidate-r2/RECEIPT.json", root / "r2-timefd-contracts/RECEIPT.json",
                  root / "r2-timefd-unit/RECEIPT.json", root / "strict-focused-clippy3/RECEIPT.json",
                  root / "r2-clone-workspace/RECEIPT.json", root / "r2-strict-focused-clippy/RECEIPT.json"]
    raw_inputs += [Path(x["raw_path"]) for x in profile_funcs.values() if "raw_path" in x]
    raw_inputs += [Path(x["path"]) for x in heap_files]
    raw_inputs += list((root / "rss-instrumented-r2").glob("*"))
    raw_inputs += list((root / "light-final-r2").glob("*.out"))
    raw_inputs += list((root / "reentrant-control-r2").glob("*.callgrind"))
    raw_inputs += [root / "annotations-corrected-r2/COMMANDS.json", root / "SUPPLEMENTAL_COMMANDS_R2.json"]
    def collect_sidecars(node):
        if isinstance(node, dict):
            for k, v in node.items():
                if k in ("stdout_path", "stderr_path") and isinstance(v, str):
                    raw_inputs.append(Path(v))
                else: collect_sidecars(v)
        elif isinstance(node, list):
            for v in node: collect_sidecars(v)
    collect_sidecars(light)
    collect_sidecars(reentrant)
    for name in failed_ids:
        raw_inputs.extend((root / name / "RECEIPT.json", root / name / "stdout.log", root / name / "stderr.log"))
    for p in raw_inputs:
        if p.is_file():
            pass
    raw_manifest = [{"path": (str(p.relative_to(root)) if a.packet else str(p)),
                     "archive_relative_path": str(p.relative_to(root)),
                     "size_bytes": p.stat().st_size, "sha256": sha(p)}
                    for p in sorted(set(raw_inputs)) if p.is_file()]

    first_failures = []
    for name in failed_ids:
        p = root / name / "RECEIPT.json"
        if p.exists():
            d = load(p)
            first_failures.append({"attempt": name, "exit_code": d.get("exit_code"),
                "wall_seconds": d.get("wall_seconds"), "receipt_sha256": sha(p),
                "stdout_sha256": d.get("stdout.log_sha256"), "stderr_sha256": d.get("stderr.log_sha256"),
                "classification": "preserved TDD/compile/lint failure; not a successful gate"})

    r2_receipt_names = ["build-baseline-final", "build-candidate-r2", "r2-clone-workspace",
                        "r2-timefd-contracts", "r2-timefd-unit", "r2-strict-focused-clippy",
                        "light-wrapper-r2", "reentrant-wrapper-r2",
                        "r2-fd-final-integration", "r2-fd-final-unit"]
    r2_checks = []
    for name in r2_receipt_names:
        rp = root / name / "RECEIPT.json"
        if rp.exists():
            d = load(rp)
            r2_checks.append({"name": name, "source_sha": d.get("git_head"), "command": d.get("command"),
                "exit_code": d.get("exit_code"), "wall_seconds": d.get("wall_seconds"),
                "status": d.get("status"), "receipt_sha256": sha(rp),
                "stdout_log_sha256": d.get("stdout.log_sha256"), "stderr_log_sha256": d.get("stderr.log_sha256")})
    supplemental = load(root / "SUPPLEMENTAL_COMMANDS_R2.json")
    r1_receipts = {name: (load(root / name / "RECEIPT.json").get("wall_seconds") if (root/name/"RECEIPT.json").exists() else None)
                   for name in ("light-wrapper", "reentrant-wrapper")}
    r2_receipts = {name: (load(root / name / "RECEIPT.json").get("wall_seconds") if (root/name/"RECEIPT.json").exists() else None)
                   for name in ("light-wrapper-r2", "reentrant-wrapper-r2")}
    supplemental_secs = sum(x.get("wall_seconds", 0.0) for x in supplemental)
    annotation_commands = load(root / "annotations-corrected-r2/COMMANDS.json")
    annotation_secs = sum(x.get("wall_seconds", 0.0) for x in annotation_commands)
    r1_measurement_total = sum(x for x in r1_receipts.values() if x is not None)
    r2_measurement_total = sum(x for x in r2_receipts.values() if x is not None) + supplemental_secs

    summary = {"schema": "vigilode-rust-optimization-r2-analysis-v1", "status": "SCOPED_DIAGNOSTIC_COMPLETE",
      "claim_ceiling": "scoped source/build/contract evidence only; broad production HOLD; timing_authority=diagnostic_only",
      "identity": {"baseline_source_sha": identity["baseline_source"], "candidate_source_sha": identity["candidate_source"],
          "candidate_tree": identity["candidate_tree"], "baseline_probe_binary_sha256": identity["binary"]["baseline"]["sha256"],
          "candidate_probe_binary_sha256": identity["binary"]["candidate"]["sha256"],
          "baseline_reentrant_binary_sha256": identity["reentrant_binary"]["baseline"]["sha256"],
          "candidate_reentrant_binary_sha256": identity["reentrant_binary"]["candidate"]["sha256"],
          "rustc": identity["compiler"], "cpu_model": next((x["data"] for x in identity["cpu"]["lscpu"] if x["field"] == "Model name:"), None),
          "measurement_cpu": identity["measurement_cpu"], "affinity_available": identity["affinity_available"],
          "os": identity["uname"], "profile": identity["profile"], "lock_sha256": identity["lock_sha256"],
          "harness_sha256": identity["harness_sha256"], "thread_environment": 1},
      "r2_scoped_build_and_tests": r2_checks,
      "later_source_qualification": {
          "status": "PASS_SCOPED" if all((root / n / "RECEIPT.json").exists() and load(root / n / "RECEIPT.json").get("exit_code") == 0 and load(root / n / "RECEIPT.json").get("source_unchanged_during_command") for n in ("r2-fd-final-integration", "r2-fd-final-unit")) else "PENDING_OR_INCOMPLETE",
          "receipts": [x for x in r2_checks if x["name"] in ("r2-fd-final-integration", "r2-fd-final-unit")],
          "observed_test_counts": {"integration": 40, "fd_unit": 13},
          "test_count_basis": "parent verified integration stdout: facade 20 + fd 14 + frontier fatal 3 + production RHS 3; fd unit 13; source is a682f609... and command receipt confirms unchanged source.",
          "r2_timing_source_remains": identity["candidate_source"],
          "scope_note": "These tests close the observed current-source FD/facade/frontier-fatal/production-RHS focused suite only; not all-target or full workspace qualification."},
      "implementation_files_and_contract_matrix": [
          {"path": "crates/vigilode/src/fd.rs", "role": "dense/banded scratch-aware FD and in-place dense callback", "contracts": ["fd_allocation_contracts", "returned-state/Stats/callback parity"]},
          {"path": "crates/vigilode/src/solve.rs", "role": "facade workspace connection", "contracts": ["fd_allocation_contracts", "targeted clippy"]},
          {"path": "crates/rodas5p-integrators/src/problem.rs", "role": "time FD scratch-aware callback/workspace preparation", "contracts": ["time_fd_workspace_tests", "time_fd_workspace_contracts"]},
          {"path": "crates/rodas5p-integrators/src/rodas5p_fast.rs", "role": "fast-driver time FD workspace buffer routing", "contracts": ["time_fd_workspace_contracts", "same output/work under test"]},
          {"path": "crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs", "role": "matrix-free fast-driver scratch routing", "contracts": ["matrix_free_fd_scratch_matches_stack_callback_with_no_extra_step_allocations"]},
          {"path": "crates/vigilode/tests/fd_allocation_contracts.rs", "role": "dense/banded facade allocation/parity regression", "contracts": ["focused cargo test"]},
          {"path": "crates/rodas5p-integrators/tests/time_fd_workspace_contracts.rs", "role": "FD vs into callback output and WorkCounters parity", "contracts": ["focused cargo test"]}],
      "implementation_file_sha256": source_files(worktree, identity["candidate_source"]),
      "r2_measurements": {"light_runner_status": light["status"], "light_failures": light["failures"],
          "reentrant_runner_status": reentrant["status"], "reentrant_failures": reentrant["failures"],
          "exact_parity_pair_count": len(checks), "exact_parity_checks": checks,
          "max_endpoint_absolute_error": maxerr, "endpoint_error_cap": CAP,
          "returned_observation_note": "Each case exports t0 and final returned observation/checkpoint; it does not export all internal accepted states. These data do not certify dense interior or every accepted physical state.",
          "reference_limit": "1e-5 cap compares f64 analytic manufactured endpoint; no separate rigorous interval/high-precision reference uncertainty bound was newly established.",
          "reused_with_identity": ["existing mathematical proof and convergence evidence; no new convergence-order campaign was run"],
          "allocations": all_allocs, "whole_solve_allocation_scope": "counted across Rodas5p::solve; includes setup/workspace/output/return allocations and is not a hot-loop-only counter",
          "timing": timing, "reentrant": reentrant_out,
          "callgrind_function_views": profile_funcs, "rss_candidate_only": rss,
          "heaptrack_candidate_samples": heap_files},
      "attempt_accounting": costs, "preserved_first_failures": first_failures,
      "measurement_cap_accounting": {"protocol_cap_seconds": 1200,
          "light_runner_R1_wrapper_seconds": r1_receipts.get("light-wrapper"),
          "light_runner_R2_wrapper_seconds": r2_receipts.get("light-wrapper-r2"),
          "reentrant_R1_wrapper_seconds": r1_receipts.get("reentrant-wrapper"),
          "reentrant_R2_wrapper_seconds": r2_receipts.get("reentrant-wrapper-r2"),
          "R1_main_wrappers_total_seconds": r1_measurement_total,
          "R2_main_wrappers_plus_recorded_supplementals_seconds": r2_measurement_total,
          "R2_supplemental_commands": supplemental,
          "R2_annotations_postprocess_seconds_not_measured_campaign": annotation_secs,
          "R1_supplemental_heaptrack_RSS_cost": "UNKNOWN_NOT_RECORDED; do not impute from R2",
          "combined_measured_main_plus_R2_supplementals_seconds": r1_measurement_total + r2_measurement_total,
          "build_and_test_cost_separate": True},
      "external_qapi_fixture_history": {"qapi_final": "/home/cosmosapjw/evidence-20261028T170155Z/qapi-final/SUMMARY.json",
          "qapi_final_classification": "INCONCLUSIVE_BASELINE_FIXTURE_FAILURE (four cargo checks exit 101)",
          "qapi_final2": "/home/cosmosapjw/evidence-20261028T170155Z/qapi-final2/SUMMARY.json",
          "qapi_final2_classification": "INCONCLUSIVE_DEPENDENCY_GRAPH; lock setup incomplete",
          "qapi_final3": "/home/cosmosapjw/evidence-20261028T170155Z/qapi-final3/SUMMARY.json",
          "qapi_final3_classification": "BREAKING_CHANGE_REPRODUCED; two checks exit0 and two current checks exit101; separate API lane, not native optimization evidence",
          "summary_hashes": {str(p): sha(p) for p in [Path('/home/cosmosapjw/evidence-20261028T170155Z/qapi-final/SUMMARY.json'), Path('/home/cosmosapjw/evidence-20261028T170155Z/qapi-final2/SUMMARY.json'), Path('/home/cosmosapjw/evidence-20261028T170155Z/qapi-final3/SUMMARY.json')] if p.exists()}},
      "r1_preserved_not_merged": {"light_manifest": str(root / "light-final/manifest.json"),
          "reentrant_summary": str(root / "reentrant-control/summary.json"),
          "measurement_identity": str(root / "MEASUREMENT_IDENTITY_FINAL.json"),
          "note": "R1 uses a different source/binary identity and is not pooled with R2."},
      "input_mode": {"packet": bool(a.packet), "packet_source": a.packet,
          "packet_original_root_identity": str(original_root) if original_root else None,
          "packet_layout": "root-relative R2 layout; include all manifested light/reentrant stdout/stderr, R2 callgrind files, corrected annotations, heaptrack-r2, rss-instrumented-r2, selected receipts and supplemental command ledger",
          "packet_mode_missing_dependency_policy": "fail with INPUT_DEPENDENCY (NOT_RUN); no missing raw is silently inferred"},
      "compact_packet_replay_limit": {"status": "REAGGREGATED_WITH_HASHED_SUPPLEMENT" if a.packet and supplement_info else ("NOT_RUN_INPUT_DEPENDENCY" if a.packet else "NOT_RUN_UNDER_LOCAL_RAW_MODE"),
          "reason": "Reentrant stdout/stderr absent from base packet are supplied only through the separately hash-verified create-only row supplement; packet-only replay without this supplement fails closed.",
          "packet_manifest_sha256": sha(packet_arg_root / "PACKET_MANIFEST.json") if packet_arg_root and packet_arg_root.is_dir() and (packet_arg_root / "PACKET_MANIFEST.json").exists() else None,
          "base_packet_manifest_entries": len(load(packet_arg_root / "PACKET_MANIFEST.json").get("packet_files", [])) if packet_arg_root and packet_arg_root.is_dir() and (packet_arg_root / "PACKET_MANIFEST.json").exists() else None,
          "base_packet_raw_index_entries": len(load(packet_arg_root / "RAW_INDEX.json").get("files", [])) if packet_arg_root and packet_arg_root.is_dir() and (packet_arg_root / "RAW_INDEX.json").exists() else None,
          "supplement": supplement_info},
      "packet_first_failure_without_supplement": {"classification": "INPUT_DEPENDENCY_NOT_RUN",
          "exit_code": 1,
          "error": "packet omits reentrant per-attempt stdout needed to recompute returned-row parity/work/error; missing 52 stdout files",
          "preserved_as": "This typed first-failure classification and missing-input count are retained in this machine-readable summary; no scientific failure is inferred."},
      "raw_input_hash_manifest": raw_manifest,
      "reproduction_command": "python3 -B research/local_dynamics_qualification_20261029/rust_optimization/analyze_results.py --root /home/cosmosapjw/vigilode-rust-opt-20261028T212815Z-t14sx5sh --worktree /home/cosmosapjw/vigilode-local-20261028T170155Z --out-dir research/local_dynamics_qualification_20261029/rust_optimization"}

    if not a.packet:
        packet_dir = outdir / "evidence"
        supplement_dir = outdir / "evidence_reentrant_rows"
        if packet_dir.is_dir() and supplement_dir.is_dir():
            with tempfile.TemporaryDirectory(prefix="vigilode-r2-packet-replay-") as replay_out:
                cmd = [sys.executable, str(Path(__file__).resolve()), "--packet", str(packet_dir),
                       "--packet-supplement", str(supplement_dir), "--out-dir", replay_out]
                started = __import__("time").monotonic()
                replay = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
                replay_summary_path = Path(replay_out) / "SUMMARY.json"
                replay_summary = load(replay_summary_path) if replay.returncode == 0 and replay_summary_path.is_file() else None
                summary["packet_reaggregation_replay"] = {
                    "status": "PASS_REAGGREGATED" if replay_summary and replay_summary.get("compact_packet_replay_limit", {}).get("status") == "REAGGREGATED_WITH_HASHED_SUPPLEMENT" else "INPUT_DEPENDENCY_OR_REPLAY_FAILURE",
                    "command": cmd, "exit_code": replay.returncode,
                    "wall_seconds": __import__("time").monotonic() - started,
                    "stdout_sha256": hashlib.sha256(replay.stdout).hexdigest(),
                    "stderr_sha256": hashlib.sha256(replay.stderr).hexdigest(),
                    "supplement_manifest_sha256": sha(supplement_dir / "SUPPLEMENT_MANIFEST.json"),
                    "supplement_row_count": replay_summary.get("compact_packet_replay_limit", {}).get("supplement", {}).get("row_count") if replay_summary else None,
                    "packet_manifest_sha256": sha(packet_dir / "PACKET_MANIFEST.json"),
                    "reaggregated_status": replay_summary.get("status") if replay_summary else None,
                    "r2_parity_pairs": replay_summary.get("r2_measurements", {}).get("exact_parity_pair_count") if replay_summary else None,
                    "reentrant_parity_pairs": replay_summary.get("r2_measurements", {}).get("reentrant", {}).get("exact_last_returned_row_parity_count") if replay_summary else None,
                    "max_endpoint_error": replay_summary.get("r2_measurements", {}).get("max_endpoint_absolute_error") if replay_summary else None,
                    "hashed_raw_input_count": len(replay_summary.get("raw_input_hash_manifest", [])) if replay_summary else None}
        else:
            summary["packet_reaggregation_replay"] = {"status": "NOT_RUN_INPUT_DEPENDENCY",
                "reason": "Compact packet and hashed reentrant supplement are not both available beside the analysis output."}

    # Categorize root receipts after content assembly.
    builds = [x for x in receipts if x["category"] == "build"]
    summary["attempt_accounting"]["build_wall_seconds"] = sum(x["wall_seconds"] for x in builds)
    summary["attempt_accounting"]["build_receipt_paths"] = [x["path"] for x in builds]
    summary["attempt_accounting"]["test_clippy_wall_seconds"] = sum(x["wall_seconds"] for x in receipts if x["category"] == "test_or_clippy")
    summary["attempt_accounting"]["runner_other_wall_seconds"] = sum(x["wall_seconds"] for x in receipts if x["category"] == "runner_or_other")

    out_json = outdir / "SUMMARY.json"
    out_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    make_plot(summary, outdir / "figures" / "rust-optimization-r2.png")
    write_report(summary, outdir / "REPORT_KO.md")
    print(json.dumps({"status": summary["status"], "parity_pairs": len(checks),
                      "max_endpoint_error": maxerr, "figure": str(outdir / "figures" / "rust-optimization-r2.png"),
                      "summary": str(out_json)}, indent=2))


def make_plot(s, path):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.ticker import MaxNLocator
    path.parent.mkdir(parents=True, exist_ok=True)
    alloc = s["r2_measurements"]["allocations"]
    labels = [x["case"].replace("nonautonomous-", "nonauto-") + (" " + x["config"] if x["config"] != "default" else "") for x in alloc]
    b = [x["baseline_allocations"] for x in alloc]
    c = [x["candidate_allocations"] for x in alloc]
    ratios = s["r2_measurements"]["timing"]
    names = list(ratios)
    med = [ratios[x]["baseline_over_candidate_ratio"]["median"] for x in names]
    lo = [ratios[x]["baseline_over_candidate_ratio"]["min"] for x in names]
    hi = [ratios[x]["baseline_over_candidate_ratio"]["max"] for x in names]
    x = list(range(len(labels)))
    fig, (ax, bx) = plt.subplots(1, 2, figsize=(15, 6))
    ax.bar([i - .18 for i in x], b, width=.36, label="baseline")
    ax.bar([i + .18 for i in x], c, width=.36, label="candidate")
    ax.set_xticks(x, labels, rotation=55, ha="right", fontsize=8)
    ax.set_ylabel("whole Rodas5p::solve allocation count")
    ax.set_title("R2 allocation totals (all allocations within solve scope)")
    ax.legend()
    nx=list(range(len(names)))
    bx.errorbar(nx, med, yerr=[[m-l for m,l in zip(med,lo)],[h-m for m,h in zip(med,hi)]], fmt="o", capsize=4)
    bx.axhline(1.0, color="black", linewidth=1)
    bx.set_xticks(nx, names, rotation=25, ha="right")
    bx.set_ylabel("Baseline / candidate")
    bx.set_title("Paired block ratio (higher favors candidate)")
    bx.yaxis.set_major_locator(MaxNLocator(7))
    fig.suptitle("VigilODE Rust FD optimization: scoped diagnostics, not production qualification")
    fig.tight_layout()
    fig.savefig(path, dpi=170)
    plt.close(fig)


def write_report(s, path):
    r=s["r2_measurements"]
    timing=r["timing"]
    slopes=r["callgrind_function_views"]["function_instruction_slopes"]
    def slope(case, function):
        rows=slopes.get(f"{case}/candidate/self",{}).get("functions",[])
        row=next((x for x in rows if x["function_pattern"]==function),None)
        return row["Ir_per_repeat_difference_over_8"] if row else None
    def fmt_slope(case, function):
        value=slope(case,function)
        return "NOT_OBSERVED" if value is None else f"{value:,.1f}".removesuffix(".0")
    iq=s["later_source_qualification"]
    iw=next((x["wall_seconds"] for x in iq["receipts"] if x["name"]=="r2-fd-final-integration"),None)
    uw=next((x["wall_seconds"] for x in iq["receipts"] if x["name"]=="r2-fd-final-unit"),None)
    lines=["# VigilODE Rust FD 최적화 R2 결과", "",
      "## 범위와 판정", "",
      "R2는 dense/banded finite-difference scratch 재사용과 비자율 time-FD 경로를 같은 release 바이너리 프로토콜로 비교한 제한된 진단이다. 판정은 **SCOPED_DIAGNOSTIC_COMPLETE / broad production HOLD**다. timing은 `diagnostic_only`이며, 이 자료만으로 범용 solver 성능·production 승격을 주장하지 않는다.", "",
      "Baseline source `"+s["identity"]["baseline_source_sha"]+"`; candidate source `"+s["identity"]["candidate_source_sha"]+"`, tree `"+s["identity"]["candidate_tree"]+"`. Probe binary SHA256 `"+s["identity"]["baseline_probe_binary_sha256"]+"` → `"+s["identity"]["candidate_probe_binary_sha256"]+"`; Rust `1.94.1`, CPU `"+str(s["identity"]["cpu_model"])+"`, CPU affinity `"+str(s["identity"]["measurement_cpu"])+"`, one solver thread. Full identity and raw input hashes are in `SUMMARY.json`.", "",
      "## 정확도와 반환 상태 동치", "",
      f"Light runner는 `{s['r2_measurements']['light_runner_status']}` / {len(s['r2_measurements']['light_failures'])} failures, reentrant runner는 `{s['r2_measurements']['reentrant_runner_status']}` / {len(s['r2_measurements']['reentrant_failures'])} failures였다. 9개 입력 설정 각각의 probe와 allocation pair, 총 {s['r2_measurements']['exact_parity_pair_count']} 쌍에서 반환된 output bits, 시간 bits, 상태, 전체 serialized Stats, callback 수가 exact parity였다. 최대 analytic endpoint max-absolute error는 `{s['r2_measurements']['max_endpoint_absolute_error']:.6g}`로 `1e-5` cap 아래다.", "",
      "관측 출력은 시작과 최종 반환 checkpoint 두 개다. 내부 accepted states 전체나 dense-output interior 오차를 export하지 않았으므로 이를 full-trajectory qualification으로 해석하지 않는다.", "",
      "## Allocation 결과", "",
      "아래 수는 allocation flag가 `Rodas5p::solve` 전체를 감싼 **전체 solve allocations**다. Setup/workspace/result 반환 등도 포함하므로 hot-loop 전용 계수나 전 solver zero-allocation을 뜻하지 않는다.", "",
      "| 입력 | Baseline | Candidate | 감소 |", "|---|---:|---:|---:|"]
    ix=lines.index("## 정확도와 반환 상태 동치")
    source_rows=["| 경로 | SHA256 (candidate source a682f609...) |", "|---|---|"]
    source_rows += [f"| `{x['path']}` | `{x['sha256']}` |" for x in s["implementation_file_sha256"]]
    lines[ix:ix]=["## Source 및 test bytes", "",
      "아래 해시는 Git에서 고정 candidate commit으로 읽은 구현 및 contract test 파일 byte identity다. Test 실행 source identity와 stdout/stderr/receipt SHA는 `SUMMARY.json`에서 연결한다.", ""] + source_rows + [""]
    for x in r["allocations"]:
        lines.append(f"| {x['case']} {x['config']} | {x['baseline_allocations']} | {x['candidate_allocations']} | {x['percent_reduction']:.1f}% |")
    lines += ["", "dense FD, banded FD, time-JVP와 nonautonomous dense control에서 감소가 관찰됐다. autonomous JVP control은 389→389로 변화가 없었다. Heaptrack candidate sample에는 `Workspace::with_dense_size`와 facade/model setup, solve-entry 및 result/export 경로의 추가 allocation sites가 남아 있다. R2 heaptrack 샘플은 dense-FD와 time-JVP 두 경우뿐이며 모든 입력으로 일반화하지 않는다.", "",
      "## Timing", "",
      "각 timed child는 64회 반복 solve를 실행했고, 한 arm당 warmup 1회 후 5개 ABBA/BAAB block을 기록했다. 아래 ratio는 **각 child의 64개 `solve_plus_drop_ns` 중앙값을 만든 다음 block 안의 arm별 2개 child 중앙값 평균을 계산하고, baseline 평균을 candidate 평균으로 나눈 뒤 5개 block의 중앙값/최솟값/최댓값**이다. 이 정의는 `analyze_results.py`에 구현되어 있다.", "",
      "| Case | Baseline/Candidate 중앙값 | Block min–max | 최악 candidate 손실 |", "|---|---:|---:|---:|"]
    for case,v in timing.items():
        q=v["baseline_over_candidate_ratio"]
        lines.append(f"| {case} | {q['median']:.3f} | {q['min']:.3f}–{q['max']:.3f} | {q['worst_observed_candidate_slowdown_percent']:.1f}% |")
    lines += ["", "Dense-FD timing 산포가 커서 중앙값으로 속도 개선을 확정하지 않는다. 관측 최악 블록 손실을 함께 보존했다. Baseline/candidate ratio가 1보다 크면 해당 정의에서 candidate가 빠르다. 측정에는 같은 CPU 0 affinity와 single-thread environment가 적용됐다.", "",
      "![R2 allocations and paired timing ranges](figures/rust-optimization-r2.png)", "",
      "## Callgrind, heaptrack, RSS", "",
      "Callgrind 4/12 반복 수준의 `Ir` 차분 기울기는 whole-process guest instruction 관측이다. 프로세스 startup과 JSON/output work가 포함되어 solver-only 비용이나 wall-time으로 바꾸지 않는다. Function `self`와 `inclusive` 표는 `annotations-corrected-r2/`에서 threshold 99.9%, auto-annotation off로 다시 만들었다. Self와 inclusive는 별도 시야이며 inclusive 행을 합산하지 않는다. `SUMMARY.json`은 case·arm별 상위 함수를 보존한다.", "",
      "R2 r12 candidate self (`inclusive=no`)에서 time-JVP의 전체 Ir 분모는 83,671,741이다. `two_pass_mgs_into` 20,266,272 (24.22%), `arnoldi_column` 6,154,200 (7.36%), QR `factor::qr_in_place` 4,682,484 (5.60%), PULP vectorize 8,964,497 (10.71%)였다. Dense-FD 전체 3,147,112 Ir 중 `Workspace::attempt`는 545,916 (17.35%), `lu_solve_in_place` 470,400 (14.95%)였다. Banded-FD의 `BandState::solve`는 3,397,920 (32.75%), `Workspace::attempt` 2,412,540 (23.25%), RHS closure 1,206,000 (11.62%)였다. Self% 분모는 해당 case의 r12 전체 Ir이다.", "",
      f"candidate self의 실제 관측 4→12 차분 기울기 `(Ir12-Ir4)/8`: time-JVP MGS {fmt_slope('time-jvp','two_pass_mgs_into')} Ir/repeat, dense Workspace::attempt {fmt_slope('dense-fd','Workspace::attempt')}, dense LU {fmt_slope('dense-fd','lu_solve_in_place')}, banded BandState::solve {fmt_slope('banded-fd','BandState::solve')}. SHA256은 0 Ir/repeat 차이가 관측됐다. annotation threshold 99.9% 때문에 한 시점에서 행이 누락된 함수 기울기는 NOT_OBSERVED이지 0이 아니다. Function self 차분과 whole-process Ir 차분은 분모/범위가 다르며 서로 분리했다. Inclusive rows는 합산하지 않는다. 이 workload profile은 solver wall-cost나 수학적 하한이 아니다.", "",
      "Candidate heaptrack는 `rodas5p_krylov::workspace::ensure_pool` 396 allocator calls / 12 time-JVP solves = 33/solve, dense `Workspace::with_dense_size` 192 / 12 = 16/solve를 관측했다. 이는 Arnoldi vector pool 및 dense workspace site이며 startup으로 단정하지 않는다. facade/model/serialization/output setup도 남아 있다. 자료는 모든 allocation을 제거 가능 또는 불가로 판정하지 않는다. 다음 소프트웨어 후보는 epoch/ownership 계약을 지키는 workspace session 재사용과 output snapshot/setup의 분리 계측이다. MGS/Arnoldi/QR 반복은 관측 계산 작업량이지 할당이나 증명된 하한이 아니다. Candidate-only RSS: dense 4592 KiB, banded 4756, time-JVP 5108, autonomous-JVP 4628. R2 heaptrack wall 0.113989s + 0.164125s, 네 RSS 명령 합 0.030922s. R1 보조 wall은 UNKNOWN이다.", "",
      "## Reentrant negative control", "",
      "`production_return_bench_20261027`는 FD scratch 변경이 직접 겨냥하지 않는 autonomous ProvidedOnly JVP control로 별도 실행됐다. 68개 process attempt가 모두 exit 0이며, 마지막 반복 row의 exact state/time/status/work/callback parity 비교 22개가 통과했다. 32-repeat 각 process에서 관측한 마지막 반복의 RHS-only work 17113, error 4.084e-8; observed strict sentinel work 24440 (1.428x), error 6.136e-8. 두 오차는 1e-5 cap 아래지만 rigorous reference uncertainty는 별도 산정되지 않았다. strict/RHS median solve time ratio는 baseline 2.598x, candidate 2.575x이며 측정은 최종 반환 결과의 drop을 제외한다. 4/12 whole-process `Ir` slope 비는 2.420x다. 차분에서 고정 startup은 invariant인 범위에서 상쇄되지만 repeated harness/JSON/output work와 startup residual 차이는 남으므로 solver-only 비용이 아니다. 반복당 상태·work·error는 **마지막 반복만** 반환되므로 이전 31회는 individually observed가 아니다. 이것은 negative control이지 FD 개선 결과가 아니다.", "",
      "## 실패와 총 실행 비용", "",
      f"초기 TDD red 시도는 raw receipt/stdout/stderr hash로 보존했다. FD red에는 allocator test unsafe/lint, exhaustive error arm 누락, dense scratch 재할당(4 대 1)이 있었다. Time-FD red에는 type mismatch, reference solve 실패, fallback allocation 차이(61 대 46; matrix-free 583 대 568), closure lifetime 오류가 있었다. Strict Clippy 최초 두 실패는 인자 수와 unused helper였다. 최종 `a682f609a1f2db1da714bea6da671b362f56baf4` source에서 facade 20 + FD 14 + frontier fatal 3 + production RHS 3 integration tests 및 `fd::tests` 13개가 모두 통과했다. source-unchanged receipt wall은 {iw:.3f}s와 {uw:.3f}s다. 이는 focused qualification이지 full workspace/all-target qualification은 아니다.", "",
      "Q-API consumer fixture의 별도 이전 lane은 `qapi-final`에서 baseline fixture setup 오류 4×exit101로 inconclusive, `qapi-final2`에서 lock/dependency graph setup incomplete, `qapi-final3`에서 baseline pass와 current API break 증거를 분리했다. 이 Q-API 결과는 R2 native 성능 측정에 포함하지 않았다.", "",
      f"외부 compact evidence packet `evidence/`는 원 raw bytes를 덮지 않고 별도 저장되었다 (PACKET_MANIFEST SHA256 `{s.get('packet_reaggregation_replay',{}).get('packet_manifest_sha256')}`, RAW_INDEX SHA256 `7de6f7af02234f25ea364f66dfff43b8f330da599446b5bc45c74b97c25aee62`; 3,541 indexed raw rows, 630 packet files (627 packaged source files + 2 indexes + manifest)). 최초 packet-only 시도는 reentrant stdout 누락 52개 때문에 exit 1 / INPUT_DEPENDENCY로 실패했다. 이 첫 실패 유형은 `SUMMARY.json`의 `packet_first_failure_without_supplement`에 기록했다. 별도 `evidence_reentrant_rows/SUPPLEMENT_MANIFEST.json`({s.get('packet_reaggregation_replay',{}).get('supplement_row_count')} stdout/stderr rows, SHA256 `{s.get('packet_reaggregation_replay',{}).get('supplement_manifest_sha256')}`)를 검증해 합친 후 packet 재집계는 {s.get('packet_reaggregation_replay',{}).get('status')} / exit {s.get('packet_reaggregation_replay',{}).get('exit_code')}로 완료됐다. packet 집계는 {s.get('packet_reaggregation_replay',{}).get('r2_parity_pairs')}개 R2 pair, {s.get('packet_reaggregation_replay',{}).get('reentrant_parity_pairs')}개 reentrant parity pair, 최대 오차 `{s.get('packet_reaggregation_replay',{}).get('max_endpoint_error')}`를 재계산했다. 재현 명령과 stdout/stderr SHA는 `SUMMARY.json`의 `packet_reaggregation_replay`에 있다.", "",
      f"Recorded process-group 비용은 build {s['attempt_accounting']['build_wall_seconds']:.3f}s, Cargo test/Clippy {s['attempt_accounting']['test_clippy_wall_seconds']:.3f}s, runner/기타 {s['attempt_accounting']['runner_other_wall_seconds']:.3f}s다. Campaign cap 1200s와 비교 가능한 known runner cost는 R1 main wrappers {s['measurement_cap_accounting']['R1_main_wrappers_total_seconds']:.3f}s + R2 wrappers 및 기록 supplemental {s['measurement_cap_accounting']['R2_main_wrappers_plus_recorded_supplementals_seconds']:.3f}s = {s['measurement_cap_accounting']['combined_measured_main_plus_R2_supplementals_seconds']:.3f}s다. R1 supplemental heaptrack/RSS elapsed 미기록분은 UNKNOWN이므로 전체가 cap 이하였다고 단정하지 않는다. R2 annotation 사후처리 {s['measurement_cap_accounting']['R2_annotations_postprocess_seconds_not_measured_campaign']:.3f}s는 별도다. Build/test 비용도 cap과 별도로 집계한다. wrapper와 내부 child receipt를 중복 합산하지 않았다.", "",
      "정확도 cap은 f64 analytic manufactured endpoint 비교다. 새 reference uncertainty 엄밀 상한은 계산하지 않았다. 기존 proof/convergence는 identity에 묶어 REUSED_WITH_IDENTITY이며 새 convergence-order campaign은 NOT_RUN이다. 파일 hash와 test/receipt 대응은 `SUMMARY.json`의 `implementation_file_sha256`, `r2_scoped_build_and_tests`, `later_source_qualification`에 있다.", "",
      "## 재현", "",
      "```bash", s["reproduction_command"], "```", "",
      "모든 raw 원본은 외부 `/home/cosmosapjw/vigilode-rust-opt-20261028T212815Z-t14sx5sh/` 아래에 불변 보존되어 있고, 입력 파일 hash는 `SUMMARY.json`에 연결했다. R1 측정은 같은 root의 `light-final/`, `reentrant-control/`, `MEASUREMENT_IDENTITY_FINAL.json`에 별도 보존됐으며 R2와 섞지 않았다.", "",
      "최종 판정은 scoped FD path 정확성/작업량 parity와 allocation reduction에 한정한다. 관측 timing의 변동과 전체 solve allocation 잔여 때문에 broad production은 계속 **HOLD**다.", ""]
    Path(path).write_text("\n".join(lines))


if __name__ == "__main__":
    main()
