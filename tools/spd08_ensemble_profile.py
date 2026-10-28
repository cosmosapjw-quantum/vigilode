#!/usr/bin/env python3
"""Instruction profile of `rodas5p stiff-ensemble-run` arms (speed research node SPD08,
research/spd08_small_ensemble_lanes_20261007; see its PREREGISTRATION.md).

Two protocols, both callgrind Ir only (no wall clock):

* `--base`: the L-0041 protocol on a binary without `--repetitions`: Ir per trajectory =
  (Ir(64 members) - Ir(32 members)) / 32, with attempts and checksum of the 64-member run.
* default: for every arm, Ir(64 members, 1 repetition) twice (determinism check), Ir(64 members,
  2 repetitions) and Ir(32 members, 1 repetition); Ir per trajectory = (Ir(64, 2) - Ir(64, 1)) / 64
  (the gated figure) and the L-0041 figure (Ir(64, 1) - Ir(32, 1)) / 32 as a cross-check. With
  `--dump-members` the per-member JSON of every arm is written next to the output and hashed.

The output file is immutable: the tool refuses to overwrite it.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import subprocess
from pathlib import Path

SCHEMA = "vigilode-spd08-ensemble-profile-v1"
HERE = Path(__file__).resolve().parent


def load_stiff_profile():
    spec = importlib.util.spec_from_file_location("stiff_profile", HERE / "stiff_profile.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--rodas5p", required=True, type=Path)
    parser.add_argument("--arms", required=True)
    parser.add_argument("--members", type=int, default=64)
    parser.add_argument("--rtol", default="1e-6")
    parser.add_argument("--scratch", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--base", action="store_true")
    parser.add_argument("--dump-members", action="store_true")
    parser.add_argument("--label", default="")
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit(f"immutable output exists: {args.output}")
    if args.members % 2:
        raise SystemExit("--members must be even (the cross-check uses half of it)")
    args.scratch.mkdir(parents=True, exist_ok=True)
    profile = load_stiff_profile()
    half = args.members // 2

    def cmd(arm, members, repetitions=None, dump=None):
        out = [str(args.rodas5p), "stiff-ensemble-run", "--arm", arm, "--members", str(members),
               "--rtol", args.rtol]
        if repetitions is not None:
            out += ["--repetitions", str(repetitions)]
        if dump is not None:
            out += ["--dump-members", str(dump)]
        return out

    def ir(arm, members, repetitions, tag):
        return profile.callgrind(cmd(arm, members, repetitions), args.scratch / f"cg.{arm}.{tag}")["total"]

    arms = {}
    for arm in args.arms.split(","):
        if args.base:
            full = ir(arm, args.members, None, f"{args.members}")
            part = ir(arm, half, None, f"{half}")
            info = profile.run_json(cmd(arm, args.members))
            arms[arm] = {"ir_members": full, "ir_half_members": part,
                         "ir_per_trajectory_members_difference": (full - part) / half,
                         "attempts": info["attempts"], "checksum": info["checksum"],
                         "checksum_bits": info.get("checksum_bits")}
        else:
            one = ir(arm, args.members, 1, f"{args.members}.r1")
            one_again = ir(arm, args.members, 1, f"{args.members}.r1b")
            two = ir(arm, args.members, 2, f"{args.members}.r2")
            part = ir(arm, half, 1, f"{half}.r1")
            info = profile.run_json(cmd(arm, args.members, 1))
            entry = {"ir_members_r1": one, "ir_members_r1_repeat": one_again, "ir_members_r2": two,
                     "ir_half_members_r1": part, "deterministic": one == one_again,
                     "ir_per_trajectory": (two - one) / args.members,
                     "ir_per_trajectory_members_difference": (one - part) / half,
                     "attempts": info["attempts"], "checksum": info["checksum"],
                     "checksum_bits": info.get("checksum_bits")}
            if args.dump_members:
                dump = args.output.with_name(args.output.stem + f".members.{arm}.json")
                if dump.exists():
                    raise SystemExit(f"immutable output exists: {dump}")
                profile.run_json(cmd(arm, args.members, 1, dump))
                entry["members_dump"] = dump.name
                entry["members_dump_sha256"] = sha256(dump)
            arms[arm] = entry
        print(arm, json.dumps(arms[arm]), flush=True)

    version = subprocess.run(["valgrind", "--version"], check=True, capture_output=True, text=True).stdout.strip()
    report = {"schema": SCHEMA, "label": args.label, "protocol": "base" if args.base else "repetitions",
              "binary_sha256": sha256(args.rodas5p), "valgrind": version, "members": args.members,
              "rtol": args.rtol, "arms": arms}
    args.output.write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print("wrote", args.output)


if __name__ == "__main__":
    main()
