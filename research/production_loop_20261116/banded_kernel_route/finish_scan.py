#!/usr/bin/env python3
"""Last measurements of node V22 under the frozen re-measurement rule (PREDECLARED.json).

usage: finish_scan.py <runner binary> <step> [<step> ...]

steps, carried out in the order given:
  scan         every scan cell without an authoritative measurement and with fewer than three
               measurements is measured again (next index), in the order of the scan
  confirm      with the width W of RESULTS.json: the confirmation cell (AMENDMENT_1.json) is
               measured until one measurement is authoritative, at most three times
  d4           the facade against the direct Indexed call on ac2d 64, until one measurement is
               authoritative, at most three times
  d4-if-final  like d4, but only if W equals BANDED_SLICES_MIN_HALF_WIDTH of the source tree
               (then the runner binary already holds the final facade rule)

Every measurement goes through quiet_measure.sh (QUIET_LOAD and QUIET_CORE of the environment);
its wait looks at the host only. Whether a measurement is authoritative is the quality rule of
the pre-declaration as the parser of the results computes it. No ratio is read here, except
that the steps after `scan` need the width W, which the parser writes to RESULTS.json.
"""
import importlib.util
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("build_results", os.path.join(HERE, "build_results.py"))
br = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(br)
v21 = br.v21
MAX_MEASUREMENTS = 3


def note(text):
    with open("/proc/loadavg") as f:
        load = f.read().split()[0]
    print(time.strftime("%Y-%m-%dT%H:%M:%S%z"), "load", load, text, flush=True)


def attempts_for(width):
    return 60 if width <= 8 else 40 if width <= 16 else 24 if width <= 32 else 12


def measure(runner, family, size, attempts, index, tag, *extra):
    note(f"measure {family} {size} attempts {attempts} index {index} tag {tag} {' '.join(extra)}")
    subprocess.run(["sh", os.path.join(HERE, "quiet_measure.sh"), runner, family, str(size),
                    str(attempts), str(index), tag, *extra], check=True)
    return os.path.join(HERE, "logs", f"drive_{tag}{index}_{family}{size}.jsonl")


def step_scan(runner):
    for family, sizes in br.SCAN.items():
        for size, width in sizes.items():
            while True:
                mine = sorted((c for c in br.cells("s") if (c["family"], c["size"]) == (family, size)),
                              key=lambda c: c["index"])
                if any(c["cycles_authoritative"] for c in mine[:MAX_MEASUREMENTS]):
                    break
                if len(mine) >= MAX_MEASUREMENTS:
                    note(f"{family} {size}: {len(mine)} measurements, none authoritative")
                    break
                measure(runner, family, size, attempts_for(width), len(mine) + 1, "s")
    br.main()


def width_of_results():
    with open(os.path.join(HERE, "RESULTS.json")) as f:
        return json.load(f)["stage_a_width_scan"]["W"]


def step_confirm(runner):
    width = width_of_results()
    if width is None:
        note("no width W: the confirmation is not measured")
        return
    size = next((s for s in (8, 16, 32) if s >= width), None)
    if size is None:
        note(f"W = {width}: no confirmation size in (8, 16, 32)")
        return
    for index in range(1, MAX_MEASUREMENTS + 1):
        path = os.path.join(HERE, "logs", f"drive_c{index}_ac2dr{size}.jsonl")
        if not os.path.exists(path):
            measure(runner, "ac2dr", size, 60, index, "c")
        if v21.drive_cell(path)["cycles_authoritative"]:
            break
    br.main()


def step_d4(runner, only_if_final):
    width = width_of_results()
    if width is None:
        note("no width W: D4 is not measured")
        return
    if only_if_final:
        source = os.path.join(HERE, "..", "..", "..", "crates", "vigilode", "src", "solve.rs")
        with open(source) as f:
            constant = int(re.search(r"BANDED_SLICES_MIN_HALF_WIDTH: usize = (\d+);", f.read()).group(1))
        if constant != width:
            note(f"W = {width} but the source tree holds {constant}: D4 waits for the final binary")
            return
    for index in range(1, MAX_MEASUREMENTS + 1):
        path = os.path.join(HERE, "logs", f"drive_f{index}_ac2d64.jsonl")
        if not os.path.exists(path):
            measure(runner, "ac2d", 64, 16, index, "f", "--kernels", "direct,facade")
        entry = br.d4_entry(v21.drive_cell(path, keys=br.D4_KEYS, other="facade"))
        if entry and entry["cycles_label"] == "authoritative":
            break
    br.main()


def main():
    runner = sys.argv[1]
    for step in sys.argv[2:]:
        note(f"step {step}")
        if step == "scan":
            step_scan(runner)
        elif step == "confirm":
            step_confirm(runner)
        elif step in ("d4", "d4-if-final"):
            step_d4(runner, step == "d4-if-final")
        else:
            raise SystemExit(f"unknown step {step}")
    note("done")


if __name__ == "__main__":
    main()
