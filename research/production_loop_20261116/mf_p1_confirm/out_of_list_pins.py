#!/usr/bin/env python3
"""Loop node V25: lists, by searching the sources, every line that pins the
default stage-solve policy of the matrix-free route or the identifier of P1
and would therefore have to change with a switch of the default to P1.
Writes OUT_OF_LIST_EDITS.json (for the owner's decision; nothing is edited).

usage: python3 -I out_of_list_pins.py
"""
import json
import os
import subprocess
import sys
import time

sys.dont_write_bytecode = True
NODE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(NODE, "..", "..", ".."))

# (file, substring, reason, inside the write list of V25)
PATTERNS = [
    ("crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs", "assert_eq!(MfStageSolvePolicy::default(), MfStageSolvePolicy::Shipped);",
     "pins the enum default to the shipped policy", False),
    ("crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs", "assert_eq!(options.matrix_free_policy, MfStageSolvePolicy::Shipped);",
     "pins RoutingOptions::default().matrix_free_policy to the shipped policy", False),
    ("crates/rodas5p-integrators/tests/mf_adaptive_shipped_bits.rs", "assert_eq!(routed.routing.driver_id, RODAS5P_MF_FAST_DRIVER_ID);",
     "pins the routed default's driver identifier to v1 and (next lines) its bits to the shipped entry", False),
    ("crates/rodas5p-integrators/tests/mf_adaptive_contracts.rs", "\"rodas5p-mf-fast-transformed-v2-candidate-p1\"",
     "pins the candidate identifier of P1, which a switch replaces by rodas5p-mf-fast-transformed-v3", False),
    ("crates/vigilode/tests/capabilities_contracts.rs", "\"rodas5p-mf-fast-transformed-v1\"",
     "pins the identifier of the default matrix-free route", True),
    ("crates/vigilode/tests/facade_contracts.rs", "RODAS5P_MF_FAST_DRIVER_ID",
     "pins the facade's matrix-free driver identifier (and, near these lines, its bits to the shipped entry)", True),
    ("crates/rodas5p-integrators/src/routing.rs", "matrix_free_policy: MfStageSolvePolicy::Shipped,",
     "the default of the route itself", True),
    ("crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs", "#[default] of pub enum MfStageSolvePolicy",
     "the enum default of MfStageSolvePolicy", True),
    ("crates/rodas5p-integrators/src/rodas5p_matrix_free_fast.rs", "Self::ColumnTested => \"rodas5p-mf-fast-transformed-v2-candidate-p1\",",
     "the identifier of P1", True),
]


def main():
    lines = []
    for rel, needle, reason, allowed in PATTERNS:
        with open(os.path.join(ROOT, rel), encoding="utf-8") as handle:
            source = handle.read().splitlines()
        if needle.startswith("#[default] of "):
            # The first #[default] after the declaration of the enum.
            start = next(k for k, text in enumerate(source) if needle[len("#[default] of "):] in text)
            number = next(k for k in range(start, len(source)) if source[k].strip() == "#[default]") + 1
            lines.append({"file": rel, "line": number, "text": source[number - 1].strip() + " " + source[number].strip(),
                          "reason": reason, "inside_the_write_list_of_V25": allowed})
            continue
        for number, text in enumerate(source, start=1):
            if needle in text:
                lines.append({"file": rel, "line": number, "text": text.strip(), "reason": reason,
                              "inside_the_write_list_of_V25": allowed})
    record = {
        "schema": "vigilode.loop-l1.v25.out-of-list-edits.v1",
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S%z") + " by out_of_list_pins.py at "
                     + subprocess.run(["git", "-C", ROOT, "rev-parse", "HEAD"], capture_output=True, text=True, check=False).stdout.strip(),
        "what": "lines that pin the shipped default of the matrix-free route or P1's candidate identifier; a switch of the default to P1 changes every one of them. Lines with inside_the_write_list_of_V25 false cannot be edited by this node, so R cannot hold after a switch made inside the write list (PROTOCOL.json known_obstacles).",
        "lines": lines,
        "outside_the_write_list": sum(1 for line in lines if not line["inside_the_write_list_of_V25"]),
    }
    with open(os.path.join(NODE, "OUT_OF_LIST_EDITS.json"), "w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=1, allow_nan=False)
        handle.write("\n")
    print(f"wrote OUT_OF_LIST_EDITS.json: {len(lines)} lines, {record['outside_the_write_list']} outside the write list")


if __name__ == "__main__":
    main()
