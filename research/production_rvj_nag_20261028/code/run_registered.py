#!/usr/bin/env python3
"""Run one source-bound native RVJ cohort without overwriting historical evidence."""
from __future__ import annotations

import argparse
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import time
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[3]
NODE = Path(__file__).resolve().parents[1]


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--source-sha", required=True)
    ap.add_argument("--remote-ref", default="origin/research/production-rvj-nag-20261028")
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--cargo", default="cargo")
    ap.add_argument("--lock", type=Path, default=Path("/tmp/vigilode-cargo-20261027.lock"))
    args = ap.parse_args()
    source = git("rev-parse", "HEAD")
    if len(args.source_sha) != 40 or source != args.source_sha:
        raise SystemExit("Source must be the exact current full SHA")
    remote = git("rev-parse", args.remote_ref)
    if remote != source:
        raise SystemExit("Fetch/read back the exact published execution source first")
    paths = ["Cargo.toml", "Cargo.lock", "crates", str(NODE.relative_to(ROOT) / "code"),
             str(NODE.relative_to(ROOT) / "theory"), str(NODE.relative_to(ROOT) / "PREREGISTRATION.md")]
    if subprocess.run(["git", "diff", "--quiet", source, "--", *paths], cwd=ROOT).returncode:
        raise SystemExit("Execution sources differ from the published commit")
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    source_files = [ROOT / "Cargo.toml", ROOT / "Cargo.lock",
                    ROOT / "crates/rodas5p-core/src/directed.rs",
                    ROOT / "crates/vigilode-coupled/src/lib.rs",
                    ROOT / "crates/vigilode-coupled/src/cyclic_closure.rs",
                    ROOT / "crates/vigilode-coupled/src/trace_quotient.rs",
                    ROOT / "crates/vigilode-coupled/examples/rvj_closure_campaign.rs",
                    ROOT / "crates/vigilode-coupled/examples/rvj_trace_campaign.rs",
                    NODE / "PREREGISTRATION.md", *sorted((NODE / "code").glob("*.py")),
                    NODE / "code/SYMBOLIC_CHECKS.wl", *sorted((NODE / "theory").glob("*"))]
    hashes = {str(p.relative_to(ROOT)): sha(p) for p in source_files if p.is_file()}
    env = os.environ.copy()
    env.update(CARGO_BUILD_JOBS="2", RAYON_NUM_THREADS="1", OMP_NUM_THREADS="1",
               OPENBLAS_NUM_THREADS="1")
    target = Path(env.get("CARGO_TARGET_DIR", str(ROOT / "target"))) / "measurement"
    receipt = {"schema": "vigilode.rvj.native-run.v1", "source_sha": source,
               "remote_ref": args.remote_ref, "remote_sha": remote,
               "source_tree": git("rev-parse", "HEAD^{tree}"), "inputs_sha256": hashes,
               "started_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
               "started_kst": dt.datetime.now(ZoneInfo("Asia/Seoul")).isoformat(),
               "platform": platform.platform(), "python": sys.version,
               "environment": {k: env.get(k) for k in ["CARGO_BUILD_JOBS", "CARGO_TARGET_DIR",
                    "RUSTFLAGS", "RUSTC", "RAYON_NUM_THREADS", "OMP_NUM_THREADS"]},
               "commands": [], "status": "RUNNING"}
    receipt_file = out / "RUN_RECEIPT.json"

    def save() -> None:
        receipt_file.write_text(json.dumps(receipt, indent=2) + "\n")

    def run(name: str, argv: list[str], cap: int, stdout_name: str | None = None) -> None:
        stdout = out / (stdout_name or name + ".stdout.txt")
        stderr = out / (name + ".stderr.txt")
        entry = {"id": name, "argv": argv, "timeout_seconds": cap,
                 "started_utc": dt.datetime.now(dt.timezone.utc).isoformat()}
        receipt["commands"].append(entry)
        save()
        print("START", name, flush=True)
        start = time.perf_counter()
        with stdout.open("xb") as so, stderr.open("xb") as se:
            proc = subprocess.Popen(argv, cwd=ROOT, env=env, stdout=so, stderr=se,
                                    start_new_session=True)
            try:
                entry["exit_code"] = proc.wait(timeout=cap)
                entry["timeout"] = False
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
                entry["exit_code"] = None
                entry["timeout"] = True
        entry.update(elapsed_seconds=time.perf_counter()-start,
                     stdout_file=stdout.name, stderr_file=stderr.name,
                     stdout_sha256=sha(stdout), stderr_sha256=sha(stderr))
        save()
        print("END", name, entry["exit_code"], entry["elapsed_seconds"], flush=True)
        if entry["exit_code"] != 0:
            raise RuntimeError(f"{name} failed; preserve this directory before any repair")

    save()
    try:
        with args.lock.open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            run("cargo-version", [args.cargo, "-V"], 30)
            run("rustc-version", [env.get("RUSTC", "rustc"), "-Vv"], 30)
            common = ["--locked", "--offline", "--profile", "measurement", "-p", "vigilode-coupled"]
            for module in ["cyclic_closure", "trace_quotient"]:
                run("test-" + module, [args.cargo, "test", *common, "--lib", module + "::", "--", "--nocapture"], 900)
            run("build-examples", [args.cargo, "build", *common, "--example", "rvj_closure_campaign",
                                    "--example", "rvj_trace_campaign"], 900)
            for family in ["closure", "trace"]:
                binary = target / "examples" / ("rvj_" + family + "_campaign")
                receipt.setdefault("binaries_sha256", {})[str(binary)] = sha(binary)
                run("native-" + family, [str(binary)], 180, family + ".json")
            run("exact-oracle", [sys.executable, str(NODE / "code/verify_native.py"),
                    "--closure", str(out / "closure.json"), "--trace", str(out / "trace.json"),
                    "--output", str(out / "ORACLE_RESULTS.json")], 900)
        changed = [p for p, digest in hashes.items() if sha(ROOT / p) != digest]
        if changed:
            raise RuntimeError("Execution source changed during run: " + repr(changed))
        receipt["status"] = "PASS"
    except Exception as exc:
        receipt["status"] = "FAIL"
        receipt["failure"] = str(exc)
    receipt["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    receipt["finished_kst"] = dt.datetime.now(ZoneInfo("Asia/Seoul")).isoformat()
    receipt["outputs_sha256"] = {p.name: sha(p) for p in sorted(out.iterdir()) if p.is_file() and p != receipt_file}
    save()
    print(json.dumps({"status": receipt["status"], "receipt": str(receipt_file)}))
    return 0 if receipt["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
