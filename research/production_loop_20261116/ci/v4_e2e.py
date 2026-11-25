#!/usr/bin/env python3
"""Local end-to-end simulation of CI v4 (node V33, owner decision OD-L1-29) with real cargo,
coverage instrumentation and strace, on a tiny selection.

  python3 -I v4_e2e.py run <scratch dir>   runs the steps below; raw outputs in v4_logs/e2e/
  python3 -I v4_e2e.py summary             parses them into v4_logs/e2e/summary.json

Steps of `run`:
  0   warm-up: the build commands of run 1, so that its budget ends in a test command
  1   run 1: rodas5p-core residual_gain_contract; rodas5p-integrators constants_provenance_contracts,
      generic_scope_contracts, ppsw1_linear_backend_contracts (1846 s in the hosted test job of
      run 45414722991) and ptsw2b_contracts; measured, --budget-minutes 3 from its start
  2   the save step and the next baseline: run 1's outcome, map and selection become the artifact
      of a fake run 9001 behind a fake gh; `select_tests.py baseline`, then `select` at the same
      head (empty diff)
  3   run 2: that selection, measured, --budget-minutes 45 from its start
  4   the baseline and the selection after run 2 (fake run 9002)
  5   run 1b: run 1's selection without a budget; SIGTERM to the selector while
      ppsw1_linear_backend_contracts runs (the signal path of V4_NOTES R39)
Every command that runs cargo runs under the host build lock and CPU set of the task.
"""

import glob
import importlib.util
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
OUT = os.path.join(HERE, "v4_logs", "e2e")
SELECTOR = os.path.join(ROOT, "tools", "ci", "select_tests.py")
LOCK = ["flock", "-w", "14400",
        "/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad/locks/cargo_a.lock",
        "taskset", "-c", "0-5,12-17"]
TARGET_DIR = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-v33"
LLVM = ("/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-llvm/rustup/toolchains/"
        "1.94.1-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin")
CORE, INTEGRATORS = "rodas5p-core", "rodas5p-integrators"
TINY = {CORE: ["residual_gain_contract"],
        INTEGRATORS: ["constants_provenance_contracts", "generic_scope_contracts",
                      "ppsw1_linear_backend_contracts", "ptsw2b_contracts"]}
SLOW = "ppsw1_linear_backend_contracts"


def selector():
    spec = importlib.util.spec_from_file_location("select_tests_e2e", SELECTOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def environment(scratch):
    env = {k: v for k, v in os.environ.items()
           if not k.startswith("GITHUB_") and k not in ("ImageOS", "ImageVersion")
           and not (k.startswith("CARGO_TARGET_") and k.endswith("_RUNNER"))}
    env.update(CARGO_TARGET_DIR=TARGET_DIR, RUSTFLAGS="-C instrument-coverage", RUSTDOCFLAGS="-C instrument-coverage",
               CARGO_BUILD_JOBS="4", CARGO_TERM_COLOR="never", RUSTUP_TOOLCHAIN="1.94.1", VIGILODE_LLVM_BIN=LLVM,
               LLVM_PROFILE_FILE=os.path.join(scratch, "unused-profiles", "%p-%m.profraw"))
    return env


def head():
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def logged(name, cmd, env, lock=True, on_start=None):
    """Run cmd from the repository root with its output in v4_logs/e2e/<name>.log; the exit code."""
    path = os.path.join(OUT, f"{name}.log")
    full = (LOCK if lock else []) + cmd
    with open(path, "w", encoding="utf-8") as log:
        log.write(f"# command: {' '.join(full)}\n# head: {head()}\n# started: {time.strftime('%Y-%m-%dT%H:%M:%S%z')}\n")
        log.flush()
        began = time.time()
        proc = subprocess.Popen(full, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
        if on_start:
            on_start(proc)
        code = proc.wait()
        log.write(f"# exit {code} after {time.time() - began:.1f} s\n")
    print(f"{name}: exit {code} ({time.time() - began:.0f} s)", flush=True)
    return code


def selection_one(st):
    entries = {name: dict(st.new_entry(), targets=list(targets), why={t: ["e2e selection"] for t in targets})
               for name, targets in TINY.items()}
    return {"schema": st.SCHEMA_SELECTION, "head": head(), "base": None, "scope": sorted(TINY), "mode": "selected",
            "reasons": [], "environment": {"image_os": None, "image_version": None, "pr_base": None,
                                           "pr_base_payload": None},
            "changed": [], "packages": entries, "notes": ["hand-made e2e selection"], "run_last": []}


def publish(scratch, run_id, outcome, cmap, selection):
    art = os.path.join(scratch, "art", str(run_id))
    os.makedirs(art, exist_ok=True)
    for src, dst in ((outcome, "outcome.json"), (cmap, "map.json"), (selection, "selection.json")):
        with open(src, "rb") as f, open(os.path.join(art, dst), "wb") as g:
            g.write(f.read())


def lookup_and_select(st, scratch, env, name, run_ids):
    """The baseline step behind the fake gh, then the select step at the same head."""
    with open(os.path.join(scratch, "listing.json"), "w", encoding="utf-8") as f:
        json.dump({"workflow_runs": [{"id": i, "created_at": f"2026-11-25T12:00:{i % 60:02d}Z", "event": "pull_request",
                                      "head_repository": {"full_name": "o/r"}} for i in run_ids]}, f)
    folder = os.path.join(OUT, name)
    os.makedirs(folder, exist_ok=True)
    gh_env = dict(env, PATH=os.path.join(scratch, "bin") + os.pathsep + env["PATH"], GITHUB_REPOSITORY="o/r",
                  GITHUB_EVENT_NAME="pull_request")
    logged(f"{name}_baseline", [sys.executable, "-I", "-u", SELECTOR, "baseline", "--workflow", "production.yml",
                                "--artifact", "ci-test-outcome-test", "--branch", "e2e", "--run-id", "99999",
                                "--head", head(), "--out", os.path.join(folder, "baseline.json")], gh_env, lock=False)
    logged(f"{name}_select", [sys.executable, "-I", "-u", SELECTOR, "select", "--baseline",
                              os.path.join(folder, "baseline.json"), "--head", head(), "--workflow-file",
                              ".github/workflows/production.yml", "--out", os.path.join(folder, "selection.json"),
                              "--scope", CORE, INTEGRATORS], env)
    return os.path.join(folder, "baseline.json"), os.path.join(folder, "selection.json")


def run_cmd(scratch, name, selection, baseline=None, budget=None, started=None):
    cmd = [sys.executable, "-I", "-u", SELECTOR, "run", "--selection", selection, "--mode", "normal", "--measure",
           "--work-dir", os.path.join(scratch, f"work-{name}"), "--map-out", os.path.join(OUT, f"{name}_map.json"),
           "--outcome", os.path.join(OUT, f"{name}_outcome.json")]
    if baseline:
        cmd += ["--baseline", baseline]
    if budget:
        cmd += ["--budget-minutes", str(budget), "--started-at", f"{started:.3f}"]
    return cmd


def run_steps(scratch):
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(os.path.join(scratch, "bin"), exist_ok=True)
    st = selector()
    with open(os.path.join(scratch, "bin", "gh"), "w", encoding="utf-8") as f:
        f.write(st.FAKE_GH)
    os.chmod(os.path.join(scratch, "bin", "gh"), 0o755)
    env = environment(scratch)
    sel1 = os.path.join(OUT, "selection1.json")
    st.write_json(sel1, selection_one(st))

    # 0. warm-up: the two build commands of run 1
    for name, targets in TINY.items():
        code = logged(f"step0_build_{name}", ["cargo", "test", "--locked", "--no-run", "-p", name]
                      + [a for t in targets for a in ("--test", t)], env)
        if code:
            raise SystemExit(f"the warm-up build of {name} failed")

    # 1. run 1, stopped by its 3-minute budget
    started = time.time()
    logged("step1_run1", run_cmd(scratch, "run1", sel1, budget=3, started=started), env)

    # 2. save, baseline, next selection (empty diff)
    publish(scratch, 9001, os.path.join(OUT, "run1_outcome.json"), os.path.join(OUT, "run1_map.json"), sel1)
    base2, sel2 = lookup_and_select(st, scratch, env, "step2", [9001])

    # 3. run 2 with a 45-minute budget
    started = time.time()
    logged("step3_run2", run_cmd(scratch, "run2", sel2, baseline=base2, budget=45, started=started), env)

    # 4. baseline and selection after run 2
    publish(scratch, 9002, os.path.join(OUT, "run2_outcome.json"), os.path.join(OUT, "run2_map.json"), sel2)
    lookup_and_select(st, scratch, env, "step4", [9001, 9002])

    # 5. run 1b: SIGTERM to the selector while the slow target runs
    outcome_1b = os.path.join(OUT, "run1b_outcome.json")

    def terminate_when_slow_runs(proc):
        deadline = time.time() + 1800
        while time.time() < deadline and proc.poll() is None:
            try:
                with open(outcome_1b, encoding="utf-8") as f:
                    running = (json.load(f).get("running") or {}).get("units") or []
            except (OSError, ValueError):
                running = []
            if any(u.get("target") == SLOW for u in running):
                time.sleep(20)
                child = subprocess.run(["pgrep", "-P", str(proc.pid)], capture_output=True, text=True).stdout.split()
                for pid in child:  # flock's child: taskset exec'd the selector
                    os.kill(int(pid), 15)
                return
            time.sleep(1)

    logged("step5_run1b", run_cmd(scratch, "run1b", sel1), env, on_start=terminate_when_slow_runs)


def counts(entry):
    return {s: len(entry.get(s, [])) for s in ("compile", "executed", "read", "dirs", "absent")}


def summarize():
    def load(rel):
        path = os.path.join(OUT, rel)
        return json.load(open(path, encoding="utf-8")) if os.path.exists(path) else None

    def exit_of(log):
        path = os.path.join(OUT, log)
        if not os.path.exists(path):
            return None
        tail = [x for x in open(path, encoding="utf-8").read().splitlines() if x.startswith("# exit ")]
        return int(tail[-1].split()[2]) if tail else None

    runs = {}
    for name, log in (("run1", "step1_run1.log"), ("run2", "step3_run2.log"), ("run1b", "step5_run1b.log")):
        outcome, cmap = load(f"{name}_outcome.json"), load(f"{name}_map.json")
        if outcome is None:
            runs[name] = None
            continue
        runs[name] = {
            "exit_code": exit_of(log), "schema": outcome["schema"], "complete": outcome["complete"],
            "stop": outcome["stop"], "running": outcome["running"], "run_last": outcome["run_last"],
            "measured": outcome["measured"], "not_run": outcome["not_run"],
            "commands": [{"command": c["command"].split(" -p ", 1)[1], "exit_code": c["exit_code"],
                          "seconds": c["seconds"], "did_not_run_its_tests": c["did_not_run_its_tests"]}
                         for c in outcome["commands"]],
            "lost": (outcome["measurement"] or {}).get("lost"),
            "strace": (outcome["measurement"] or {}).get("strace"),
            "map_targets": {k: counts(v) for k, v in sorted((cmap or {}).get("targets", {}).items())},
            "map_heads": sorted({v.get("head") for v in (cmap or {}).get("targets", {}).values()}),
            "map_packages": sorted((cmap or {}).get("packages", {})),
            "budget": outcome["budget"],
        }
    steps = {}
    for name in ("step2", "step4"):
        base, sel = load(f"{name}/baseline.json"), load(f"{name}/selection.json")
        steps[name] = None if base is None or sel is None else {
            "baseline": {k: base.get(k) for k in ("run_id", "base", "complete", "not_run", "run_last", "note", "errors")},
            "selection": {"mode": sel["mode"], "reasons": sel["reasons"], "changed": sel["changed"],
                          "packages": {p: {k: e[k] for k in ("all", "lib", "doc", "targets", "examples")}
                                       for p, e in sel["packages"].items()},
                          "why": {p: e["why"] for p, e in sel["packages"].items()},
                          "baseline_not_run_added": sel.get("baseline_not_run_added"),
                          "baseline_not_run_dropped": sel.get("baseline_not_run_dropped"),
                          "run_last": sel.get("run_last")},
            "baseline_exit": exit_of(f"{name}_baseline.log"), "select_exit": exit_of(f"{name}_select.log")}
    result = {"schema": "vigilode.loop.ci-v4-e2e-summary.v1", "head": head(),
              "warm_up_exits": {os.path.basename(p): exit_of(os.path.basename(p))
                                for p in sorted(glob.glob(os.path.join(OUT, "step0_build_*.log")))},
              "runs": runs, "steps": steps}
    with open(os.path.join(OUT, "summary.json"), "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1, sort_keys=True, allow_nan=False)
        f.write("\n")
    print(json.dumps(result, indent=1, sort_keys=True)[:6000])
    return 0


def main():
    if sys.argv[1:2] == ["run"] and len(sys.argv) == 3:
        run_steps(os.path.abspath(sys.argv[2]))
        return summarize()
    if sys.argv[1:] == ["summary"]:
        return summarize()
    raise SystemExit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
