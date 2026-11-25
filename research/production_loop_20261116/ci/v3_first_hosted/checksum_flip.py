#!/usr/bin/env python3
"""Rebuild the scientific checksum of a `rodas5p stage-batch-feasibility` report and the checksum
with the field `parallel-observed` flipped (CI triage of 2026-11-25, section 12 of ../TRIAGE_KO.md).

The signature follows `scientific_checksum` in crates/rodas5p-integrators/src/stage_batch.rs: one line
`parallel-observed=<bool>`, one line per case and one line per row with the bits of
`max_abs_difference`. Usage:

    python3 -I checksum_flip.py <report.json> [<hosted checksum> ...]
"""
import hashlib
import json
import struct
import sys


def bits(value):
    return struct.unpack("<Q", struct.pack("<d", float(value)))[0]


def text(flag):
    return "true" if flag else "false"


def signature(report, parallel_observed, flip_row=None):
    lines = [f"parallel-observed={text(parallel_observed)}\n"]
    for case in report["cases"]:
        lines.append(f"case|{case['case_id']}|{case['family']}|{case['dimension']}|"
                     f"{case['work_repeats']}|{case['stages']}\n")
    for index, row in enumerate(report["rows"]):
        actual = row["actual_stage_parallel"] if flip_row != index else not row["actual_stage_parallel"]
        lines.append(f"row|{row['case_id']}|{row['kernel']}|{row['backend']}|{row['threads']}|{row['stages']}|"
                     f"{row['dimension']}|{row['work_repeats']}|{bits(row['max_abs_difference']):016x}|"
                     f"{text(actual)}\n")
    return hashlib.sha256("".join(lines).encode()).hexdigest()


def main():
    report = json.load(open(sys.argv[1], encoding="utf-8"))
    observed = report["stage_parallelism_observed"]
    rebuilt = signature(report, observed)
    flipped = signature(report, not observed)
    print(f"report checksum      {report['scientific_checksum']}")
    print(f"rebuilt              {rebuilt} (equal: {rebuilt == report['scientific_checksum']})")
    print(f"parallel-observed={text(not observed):5s} {flipped}")
    for hosted in sys.argv[2:]:
        row_hits = [i for i in range(len(report["rows"])) if signature(report, observed, flip_row=i) == hosted]
        print(f"hosted {hosted}: equals rebuilt {hosted == rebuilt}, equals flipped {hosted == flipped}, "
              f"single actual_stage_parallel flips {row_hits}")
    nonzero = [r for r in report["rows"] if r["max_abs_difference"] != 0]
    print(f"rows with non-zero max_abs_difference: {len(nonzero)} of {len(report['rows'])}")


if __name__ == "__main__":
    main()
