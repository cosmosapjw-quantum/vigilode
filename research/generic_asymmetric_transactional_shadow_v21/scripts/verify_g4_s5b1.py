#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ANALYSIS = json.loads((ROOT / "results/G4_S5B1_ANALYSIS.json").read_text())

assert ANALYSIS["active_switching"] is False
assert ANALYSIS["candidate_source_modified"] is False
assert len(set(ANALYSIS["scientific_hashes"].values())) == 1
metric = ANALYSIS["metric_correction"]["statistics"]
assert metric["hard_rows"] == 258
assert metric["winner_flips"] == 24
assert abs(metric["winner_flip_fraction"] - 24 / 258) < 1e-15

for policy in ("r-default", "e-first"):
    assert ANALYSIS["contract_gates"][policy]["unsafe_zero"] is True
    assert ANALYSIS["contract_gates"][policy]["activation_gate_pass"] is False

safe = ANALYSIS["results"]["safe-e-diagnostic"]
assert safe["holdout"]["median_regret"] <= 1.15
assert safe["holdout"]["p95_regret"] <= 1.5
assert safe["calibration"]["p95_regret"] > 1.5
assert ANALYSIS["closeout"]["active_switching"] == "HOLD"
print("G4_S5B1_VERIFICATION=PASS")
