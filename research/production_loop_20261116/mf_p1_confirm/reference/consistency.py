#!/usr/bin/env python3
"""The frozen consistency check of loop node V25 (PREDECLARED.json,
consistency_of_the_two_right_hand_sides): before any reference is computed,
the numpy and the Rust right-hand sides and Jacobian-vector products of each
family agree at 20 deterministic states to 1e-12 in the max norm relative to
1 + |value|.

The measurement binary (mode CONSIST) writes the 20 states y_k, directions
v_k, f(t0, y_k) and J(t0, y_k) v_k as hex to a file; this script evaluates
families.py at the same bits and compares:

  d_f  = max_i |f_rust_i  - f_numpy_i|  / (1 + |f_numpy_i|)
  d_jv = max_i |jv_rust_i - jv_numpy_i| / (1 + |jv_numpy_i|)

A family passes iff every d_f and d_jv of its 20 states is at most 1e-12.
Output: logs/consistency/<family>.log (raw lines) and CONSISTENCY.json.
The hex file itself (several MB) stays outside the repository; its sha256
is logged.

usage: python3 -I consistency.py --bin MEASURE_BINARY --scratch DIR [--families a,b]
"""
import argparse
import hashlib
import json
import os
import struct
import subprocess
import sys
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
NODE = os.path.dirname(HERE)
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))
LOGS = os.path.join(NODE, "logs", "consistency")
sys.path.insert(0, HERE)
import families  # noqa: E402  (the module next to this script)

THRESHOLD = 1e-12
STATES = 20


def unhex(text):
    return np.array([struct.unpack(">d", bytes.fromhex(text[i:i + 16]))[0] for i in range(0, len(text), 16)])


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True, check=False).stdout.strip()


def scaled(a, b):
    return float(np.max(np.abs(a - b) / (1.0 + np.abs(b))))


def check(family, binary, scratch):
    os.makedirs(LOGS, exist_ok=True)
    os.makedirs(scratch, exist_ok=True)
    out_path = os.path.join(scratch, f"{family}.consist.txt")
    env = {key: value for key, value in os.environ.items() if not key.startswith("V25_")}
    env.update({"V25_MODE": "CONSIST", "V25_FAMILY": family, "V25_CONSIST_OUT": out_path})
    command = ["taskset", "-c", "0-5,12-17", binary, "--ignored", "--nocapture", "--exact", "v25_run"]
    header = {"family": family, "binary": binary, "binary_sha256": sha256_file(binary), "git_head": git("rev-parse", "HEAD"),
              "date": time.strftime("%Y-%m-%dT%H:%M:%S%z"), "command": command, "threshold": THRESHOLD}
    completed = subprocess.run(command, env=env, capture_output=True, text=True, check=False)
    rhs, jvp, _y0, _pattern = families.BUILDERS[family]()
    rows = []
    with open(out_path, encoding="ascii") as handle:
        lines = handle.read().splitlines()
    for line in lines:
        k, y_hex, v_hex, f_hex, jv_hex = line.split(" ")
        y, v, f_rust, jv_rust = unhex(y_hex), unhex(v_hex), unhex(f_hex), unhex(jv_hex)
        f_numpy = rhs(0.0, y)
        jv_numpy = jvp(0.0, y, v)
        rows.append({
            "k": int(k), "n": int(y.size),
            "d_f": scaled(f_rust, f_numpy), "d_jv": scaled(jv_rust, jv_numpy),
            "f_bitwise_equal_components": int(np.sum(f_rust.view(np.uint64) == f_numpy.view(np.uint64))),
            "jv_bitwise_equal_components": int(np.sum(jv_rust.view(np.uint64) == jv_numpy.view(np.uint64))),
            "max_abs_f": float(np.max(np.abs(f_numpy))), "max_abs_jv": float(np.max(np.abs(jv_numpy))),
        })
    passed = len(rows) == STATES and all(row["d_f"] <= THRESHOLD and row["d_jv"] <= THRESHOLD for row in rows)
    summary = {
        "family": family, "states": len(rows), "passes": passed, "exit_code": completed.returncode,
        "max_d_f": max(row["d_f"] for row in rows) if rows else None,
        "max_d_jv": max(row["d_jv"] for row in rows) if rows else None,
        "hex_file_sha256": sha256_file(out_path), "hex_file_kept_at": out_path,
        "all_components_bitwise_equal": all(row["f_bitwise_equal_components"] == row["n"] and row["jv_bitwise_equal_components"] == row["n"] for row in rows),
    }
    with open(os.path.join(LOGS, f"{family}.log"), "w", encoding="utf-8") as log:
        log.write("# V25_CONSIST_BEGIN " + json.dumps(header) + "\n")
        log.write(completed.stdout)
        if completed.stderr:
            log.write("# stderr\n" + completed.stderr)
        for row in rows:
            log.write("V25_CONSIST_CHECK " + json.dumps(row) + "\n")
        log.write("# V25_CONSIST_END " + json.dumps(summary) + "\n")
    return header, summary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bin", required=True)
    parser.add_argument("--scratch", required=True, help="directory for the hex files (outside the repository)")
    parser.add_argument("--families", default=",".join(families.DECLARED))
    args = parser.parse_args()
    record = {"schema": "vigilode.loop-l1.v25.consistency.v1", "rule": __doc__.split("\n\n")[0].replace("\n", " "),
              "threshold": THRESHOLD, "states": STATES,
              "statistic": "max_i |rust_i - numpy_i| / (1 + |numpy_i|) for f(t0, y_k) and J(t0, y_k) v_k",
              "families": {}}
    ok = True
    for family in [f for f in args.families.split(",") if f]:
        header, summary = check(family, args.bin, args.scratch)
        record["families"][family] = {"binary_sha256": header["binary_sha256"], "git_head": header["git_head"],
                                      "log": os.path.relpath(os.path.join(LOGS, f"{family}.log"), NODE), **summary}
        ok = ok and summary["passes"]
        print(json.dumps(summary), flush=True)
    record["all_pass"] = ok
    path = os.path.join(NODE, "CONSISTENCY.json")
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=1, allow_nan=False)
        handle.write("\n")
    print(f"wrote {path}: all_pass {ok}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
