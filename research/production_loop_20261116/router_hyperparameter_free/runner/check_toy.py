#!/usr/bin/env python3
"""Plumbing check of the V27 runner (phase 1): the toy problem y' = -y over [0, 1] only.

No problem of the deciding set of PREDECLARED.json is run here. The check runs both builds
(arm N: this worktree with the feature `pilots`; arm T: the base checkout) with their own arm
and the reference arms D and R, twice each, writes every JSON line to
logs/runner_toy.jsonl and prints one summary line. It fails (exit 1) if a line does not
parse, if a repeated run differs, or if D or R differ between the two builds.

Usage: python3 -I check_toy.py
"""
import json
import os
import subprocess
import sys

TARGETS = "/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116"
BINARIES = {
    "N": f"{TARGETS}/w3-v27/release/v27-runner",
    "T": f"{TARGETS}/w3-v27base/release/v27-runner",
}
HERE = os.path.dirname(os.path.abspath(__file__))
LOG = os.path.join(HERE, "..", "logs", "runner_toy.jsonl")


def run(binary, *args):
    done = subprocess.run([binary, *args], capture_output=True, text=True)
    return done.returncode, done.stdout, done.stderr


def main():
    lines, problems = [], []
    by_key = {}
    for build, binary in BINARIES.items():
        for arm in (build, "D", "R"):
            for repeat in (1, 2):
                code, out, err = run(binary, "--arm", arm, "--problem", "toy")
                if code != 0:
                    problems.append(f"{build}/{arm}: exit {code}: {err.strip()}")
                    continue
                line = json.loads(out)
                line["check"] = {"binary": binary, "repeat": repeat}
                lines.append(line)
                by_key.setdefault((build, arm), []).append(line)
        # each build refuses the other build's arm
        other = "T" if build == "N" else "N"
        code, _, err = run(binary, "--arm", other, "--problem", "toy")
        if code != 2:
            problems.append(f"{build} accepted arm {other}")

    def essence(line):
        run_ = line["run"]
        return json.dumps(
            {
                "final_time_bits": run_.get("final_time_bits"),
                "final_state_bits": run_.get("final_state_bits"),
                "counters": run_.get("counters"),
                "attempts": run_.get("attempts"),
                "status": run_.get("status"),
                "route_digest": run_.get("route_digest"),
                "entries": line["entries"],
            },
            sort_keys=True,
        )

    for key, runs in by_key.items():
        if len({essence(r) for r in runs}) != 1:
            problems.append(f"{key}: repeated runs differ")
        if not runs[0]["run"].get("success"):
            problems.append(f"{key}: not successful")
    for arm in ("D", "R"):
        n_runs, t_runs = by_key.get(("N", arm), []), by_key.get(("T", arm), [])
        if n_runs and t_runs and essence(n_runs[0]) != essence(t_runs[0]):
            problems.append(f"arm {arm} differs between the builds")
    os.makedirs(os.path.dirname(LOG), exist_ok=True)
    with open(LOG, "w") as handle:
        for line in lines:
            handle.write(json.dumps(line, sort_keys=True) + "\n")
    summary = {
        "check": "v27 runner plumbing, toy problem only",
        "lines": len(lines),
        "problems": problems,
        "N": {k: by_key[("N", "N")][0]["run"].get(k) for k in ("success", "attempts", "route_digest")}
        if ("N", "N") in by_key
        else None,
        "T": {k: by_key[("T", "T")][0]["run"].get(k) for k in ("success", "attempts", "route_digest")}
        if ("T", "T") in by_key
        else None,
        "pilots_N": by_key[("N", "N")][0]["run"].get("pilots") if ("N", "N") in by_key else None,
    }
    print(json.dumps(summary, sort_keys=True))
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
