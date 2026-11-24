#!/usr/bin/env python3
"""Which part of a RODAS5P step dominates, per measured regime (node SL-R3).

Reads the V04 cost anatomy (`research/production_loop_20261116/cost_anatomy/AMDAHL.json`,
fractions f_J, f_LU, f_S, f_R of the instrumented step) and assigns each cell the lever
that acts on its largest fraction:

  f_R  (eight right-hand sides)      -> rhs: split forcing / row-parallel RHS / prefetch
  f_S  (eight stage solves)          -> stage-solve critical path: two-way, SPIKE, lattice
  f_LU (one factorisation)           -> factorisation: parallel LU, reuse, kernel
  f_J  (one Jacobian)                -> Jacobian: colouring, parallel assembly, AD

The "eight sequential loops" premise of the project targets f_R + f_S. The script reports
for every cell the share of the step that a perfect P-way parallelisation of the stage
solves alone could remove (Amdahl bound 1 / (1 - f_S (1 - 1/P))). Usage:
regime_map.py <repo root> <output json>
"""
from __future__ import annotations

import json
import pathlib
import sys


def main() -> int:
    root = pathlib.Path(sys.argv[1])
    amdahl = json.loads(
        (root / "research/production_loop_20261116/cost_anatomy/AMDAHL.json").read_text()
    )
    cells = []
    for r in amdahl["regimes"]:
        f = r["fractions"]
        parts = {"rhs": f["f_R"], "stage_solves": f["f_S"], "factorisation": f["f_LU"], "jacobian": f["f_J"]}
        lever = max(parts, key=parts.get)
        cells.append({
            "cell": r["cell"], "structure": r["structure"], "n": r["n"],
            "admissibility": r["admissibility"], "fractions": parts,
            "dominant": lever,
            "bound_two_way_P2": 1.0 / (1.0 - f["f_S"] * 0.5),
            # The two-way client splits the factorisation as well as the eight solves.
            "bound_two_way_P2_factor_and_solves": 1.0 / (1.0 - (f["f_S"] + f["f_LU"]) * 0.5),
            "bound_spike_P8": 1.0 / (1.0 - f["f_S"] * (1 - 1 / 8)),
        })
    by_structure: dict[str, dict[str, int]] = {}
    for c in cells:
        by_structure.setdefault(c["structure"], {}).setdefault(c["dominant"], 0)
        by_structure[c["structure"]][c["dominant"]] += 1
    out = {
        "schema": "vigilode.referee2.regime-map.v1",
        "source": "research/production_loop_20261116/cost_anatomy/AMDAHL.json",
        "dominant_by_structure": by_structure,
        "cells": cells,
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(out, indent=1) + "\n")
    for s, d in by_structure.items():
        print(s, d)
    for c in cells:
        if c["structure"].startswith("banded"):
            print(f"{c['cell']:40s} f_S={c['fractions']['stage_solves']:.3f} "
                  f"two-way P2 bound={c['bound_two_way_P2']:.3f} "
                  f"(factor+solves {c['bound_two_way_P2_factor_and_solves']:.3f}) "
                  f"spike P8 bound={c['bound_spike_P8']:.3f} [{c['admissibility'][:5]}]")
    return 0


if __name__ == "__main__":
    sys.exit(main())
