#!/usr/bin/env python3
"""Independent operational checks with temporary byte fixtures; no scientific ODE."""
from contextlib import redirect_stdout
from concurrent.futures import ProcessPoolExecutor
import hashlib
import io
import json
import multiprocessing
from pathlib import Path
import tempfile
import time
from unittest.mock import patch

import run_pilot as runner

STUB = '''from pathlib import Path
import hashlib, json, os
def run_case(config, outdir):
    outdir = Path(outdir)
    with (outdir / "stub_calls.txt").open("a") as handle: handle.write("one call\\n")
    if config.get("fail"):
        result = {"status":"FAILED", "reason":"intentional generic fixture failure"}
    else:
        payload = b"STUB BYTE FIXTURE: NO ODE OR PHYSICAL TRAJECTORY\\n"
        (outdir / "trajectory.npz.tmp").write_bytes(payload)
        os.replace(outdir / "trajectory.npz.tmp", outdir / "trajectory.npz")
        result = {"status":"COMPLETE", "scientific_identity_sha256":config["_identity"],
            "outputs":{"trajectory.npz":{"bytes":len(payload),"sha256":hashlib.sha256(payload).hexdigest()}}}
    (outdir / "RESULT.json.tmp").write_text(json.dumps(result))
    os.replace(outdir / "RESULT.json.tmp", outdir / "RESULT.json")
    return result
'''


def fixture(path):
    for name in ("code", "data", "provenance", "vendor"):
        (path / name).mkdir(parents=True)
    (path / "code/coupled.py").write_text(STUB)
    (path / "vendor/owner.py").write_text("# immutable fixture owner\n")
    runner.atomic_json(path / "provenance/BASELINE_SOURCES.json", {"files": [{"path": "vendor/owner.py", "sha256": runner.digest(path / "vendor/owner.py")}]})
    runner.atomic_json(path / "data/CAMPAIGN.json", {"identity_files": ["code/coupled.py", "vendor/*.py"],
        "cases": [{"case_id": str(k), "config": {"Sigma_F": k / 100, "matter_A": .2, "radiation_A": .1}} for k in range(3)]})
    return runner.freeze(path, path / "data/CAMPAIGN.json")


def main():
    checks, started = [], time.monotonic()
    def record(name, **measurement):
        checks.append({"name": name, "status": "PASS", **measurement})
    with tempfile.TemporaryDirectory(prefix="hheli_runner_") as temporary:
        root = Path(temporary)
        frozen = fixture(root)
        output = root / "results/pilot"
        old_root = runner.ROOT
        runner.ROOT = root
        try:
            with redirect_stdout(io.StringIO()):
                first = runner.main(["--execute", "--manifest", str(root / "data/CAMPAIGN.json"), "--output", str(output), "--limit", "1", "--workers", "2"])
            assert first == 0
            first_hash = runner.digest(output / "cases/0/RESULT.json")
            first_mtime = (output / "cases/0/RESULT.json").stat().st_mtime_ns
            with redirect_stdout(io.StringIO()):
                second = runner.main(["--execute", "--manifest", str(root / "data/CAMPAIGN.json"), "--output", str(output), "--workers", "2"])
            assert second == 0
            assert first_hash == runner.digest(output / "cases/0/RESULT.json")
            assert first_mtime == (output / "cases/0/RESULT.json").stat().st_mtime_ns
            summaries = [runner.read_json(p) for p in output.glob("RUN_*.json")]
            assert any(sum(r["status"] == "complete_reused" for r in s["records"]) == 1 for s in summaries)
            assert all((output / f"cases/{k}/stub_calls.txt").read_text().count("one call") == 1 for k in range(3))
            record("limit_one_then_full_reuses_without_reexecution", total_cases=3, reused=1, spawned_stub_calls=3)
            with patch.object(runner, "resource_budget", return_value={"workers": 0}), patch.object(runner, "ProcessPoolExecutor", side_effect=AssertionError("reuse attempted new worker")), redirect_stdout(io.StringIO()):
                third = runner.main(["--execute", "--manifest", str(root / "data/CAMPAIGN.json"), "--output", str(output)])
            assert third == 0
            record("all_complete_reuse_requires_no_worker_allocation", mocked_available_workers=0)
        finally:
            runner.ROOT = old_root

        config = {**frozen["configs"][0], "case_id": "concurrent"}
        with ProcessPoolExecutor(max_workers=2, mp_context=multiprocessing.get_context("spawn")) as pool:
            futures = [pool.submit(runner.execute_one, str(root), str(output), frozen, config) for _ in range(2)]
            statuses = [f.result()["status"] for f in futures]
        assert statuses.count("completed") == 1
        assert all(s in {"completed", "complete_reused", "existing_preserved"} for s in statuses)
        assert (output / "cases/concurrent/stub_calls.txt").read_text().count("one call") == 1
        record("atomic_single_owner_under_two_spawned_workers", observed_statuses=statuses, actual_stub_calls=1)

        failed = {**config, "case_id": "failed", "fail": True}
        failure = runner.execute_one(str(root), str(output), frozen, failed)
        assert failure["status"] == "failed_preserved"
        original = runner.digest(output / "cases/failed/RESULT.json")
        assert runner.execute_one(str(root), str(output), frozen, failed)["status"] == "existing_preserved"
        assert original == runner.digest(output / "cases/failed/RESULT.json")
        assert (output / "cases/failed/stub_calls.txt").read_text().count("one call") == 1
        record("failed_case_and_failure_receipt_preserved_without_retry")

        (output / "cases/0/trajectory.npz").write_bytes(b"intentional byte corruption")
        assert not runner.valid_completion(output / "cases/0", runner.case_identity(frozen, frozen["configs"][0]))
        assert runner.execute_one(str(root), str(output), frozen, frozen["configs"][0])["status"] == "existing_preserved"
        assert (output / "cases/0/trajectory.npz").read_bytes() == b"intentional byte corruption"
        record("corrupt_completed_payload_is_preserved_and_not_reused")

        empty = output / "cases/unknown"
        empty.mkdir()
        assert runner.execute_one(str(root), str(output), frozen, {**config, "case_id": "unknown"})["status"] == "existing_preserved"
        assert list(empty.iterdir()) == []
        record("unknown_claim_directory_never_deleted_or_taken_over")

        (root / "code/coupled.py").write_text(STUB + "\n# changed dependency\n")
        try:
            runner.verify_inputs(root, frozen)
            raise AssertionError("changed source was accepted")
        except ValueError:
            pass
        newer = runner.freeze(root, root / "data/CAMPAIGN.json")
        assert newer["base_identity"] != frozen["base_identity"]
        assert runner.case_identity(newer, config) != runner.case_identity(frozen, config)
        record("source_mutation_changes_identity_and_rejects_frozen_input")

        (root / "vendor/owner.py").write_text("# changed vendor\n")
        try:
            runner.freeze(root, root / "data/CAMPAIGN.json")
            raise AssertionError("changed pinned baseline was accepted")
        except ValueError:
            pass
        record("pinned_baseline_hash_is_checked_before_execution")

        with patch.object(runner.os, "getpid", side_effect=RuntimeError("namespace-mismatched PID")):
            identity = runner.owner()
        assert identity["token"] and "proc_pid" in identity
        record("claim_uses_proc_self_identity_without_getpid_path", observed_proc_pid=identity["proc_pid"])

        leases = root / "peer_leases"
        leases.mkdir()
        runner.atomic_json(leases / "peer.json", {**runner.owner(), "status": "RUNNING", "workers": 4, "memory_bytes": 2 * 1024**3})
        before = runner.digest(leases / "peer.json")
        original_read = Path.read_text
        def host_read(path, *args, **kwargs):
            values = {"/proc/meminfo": "MemTotal: 134217728 kB\nMemAvailable: 104857600 kB\n",
                "/sys/fs/cgroup/cpu.max": "max 100000", "/sys/fs/cgroup/memory.max": "max"}
            return values[str(path)] if str(path) in values else original_read(path, *args, **kwargs)
        with patch.object(Path, "read_text", host_read), patch.object(runner.os, "sched_getaffinity", return_value=set(range(64))), patch.object(runner.os, "getloadavg", return_value=(10.1, 9., 8.)):
            budget = runner.resource_budget([leases])
        assert budget["workers"] == 52 and budget["workers"] <= 63
        assert budget["peer_reserved_cpu"] == 4 and budget["default_worker_cap"] == 63
        assert before == runner.digest(leases / "peer.json")
        record("synthetic_64core_budget_respects_load_and_peer_reservation", workers=52, mocked_host=True, NCP_execution=0)

        (leases / "unknown.json").write_text("[]")
        budget = runner.resource_budget([leases])
        assert budget["peer_reserved_cpu"] == 5
        assert (leases / "unknown.json").read_text() == "[]"
        try:
            runner.resource_budget([], per_worker_mib=0)
            raise AssertionError("zero memory budget accepted")
        except ValueError:
            pass
        record("malformed_peer_reserved_conservatively_and_zero_memory_rejected")

        dead = {**runner.owner(), "status": "RUNNING", "workers": 63, "memory_bytes": 63 * 512 * 1024**2}
        dead["proc_pid"] = 2**31 - 1
        runner.atomic_json(leases / "dead.json", dead)
        dead_hash = runner.digest(leases / "dead.json")
        dead_budget = runner.resource_budget([leases])
        assert dead_budget["peer_reserved_cpu"] == 5
        assert any(r["path"].endswith("dead.json") and r["state"] == "dead_absent_pid" for r in dead_budget["peer_reservations"])
        assert dead_hash == runner.digest(leases / "dead.json")
        record("confirmed_dead_lease_ignored_and_bytes_preserved", dead_reserved_cpu_debit=0)

        local_budget = runner.resource_budget([])
        assert 0 <= local_budget["workers"] <= min(63, int(local_budget["effective_cpu_quota"]))
        record("actual_local_resource_observation", resources=local_budget)

    receipt = {"status": "PASS_SCOPED", "new_ODE": 0, "new_NCP": 0, "benchmark": 0,
               "scope": "temporary generic byte fixtures; dispatcher ownership, reuse and resource decisions", "checks": checks,
               "code_sha256": runner.digest(Path(runner.__file__)), "verifier_sha256": runner.digest(Path(__file__)),
               "wall_seconds": time.monotonic() - started}
    runner.atomic_json(runner.ROOT / "evidence/RUNNER_VERIFICATION.json", receipt)
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
