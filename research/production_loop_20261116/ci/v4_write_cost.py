#!/usr/bin/env python3
"""Cost of the incremental writes of CI v4 (node V33, OD-L1-29) at the size of the hosted full test
job (557 map entries, 475 commands): per command the run step writes the input map once and the
outcome twice (before and after the command). Recorded only; it decides nothing.

  python3 -I v4_write_cost.py      prints one line (v4_logs/write_cost.log)

The map entries are copies of the measured entries in v3_logs/fix_e2e_map.json.
"""

import importlib.util
import json
import os
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SELECTOR = os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "ci", "select_tests.py"))


def main():
    spec = importlib.util.spec_from_file_location("select_tests_cost", SELECTOR)
    st = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(st)
    with open(os.path.join(HERE, "v3_logs", "fix_e2e_map.json"), encoding="utf-8") as f:
        base = json.load(f)
    entries = list(base["targets"].values())
    targets = {f"p::t{i}": entries[i % len(entries)] for i in range(557)}
    cmap = {"schema": st.SCHEMA_MAP, "head": "h", "mode": "normal", "targets": targets,
            "packages": base.get("packages", {})}
    outcome = {"schema": st.SCHEMA_OUTCOME,
               "commands": [{"command": f"cargo test --locked --no-fail-fast -p p --test t{i}", "exit_code": 0,
                             "seconds": 1.0, "failed_targets": [], "did_not_run_its_tests": False, "build": False}
                            for i in range(475)],
               "measurement": {"stats": {f"p::t{i}": {"seconds": 1.0, "trace_files": 2, "traced_calls": 100,
                                                      "compile": 2, "executed": 10, "read": 3, "dirs": 0,
                                                      "absent": 0} for i in range(557)}},
               "not_run": [], "measured": sorted(targets)}
    rounds = 20
    with tempfile.TemporaryDirectory() as d:
        m, o = os.path.join(d, "map.json"), os.path.join(d, "outcome.json")
        began = time.perf_counter()
        for _ in range(rounds):
            st.write_json(m, cmap)
            st.write_json(o, outcome)
            st.write_json(o, outcome)
        per = (time.perf_counter() - began) / rounds
        print(f"map {os.path.getsize(m) / 1e6:.2f} MB, outcome {os.path.getsize(o) / 1e6:.2f} MB; writes per command "
              f"(map once, outcome twice): {per * 1000:.0f} ms (mean of {rounds}); x475 commands: {per * 475:.0f} s")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
