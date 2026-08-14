#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
analysis = json.loads((root / "results" / "G4_S4_ANALYSIS.json").read_text())

assert analysis["inputs"]["campaign_count"] == 7
assert analysis["inputs"]["scientific_identity"] is True
assert analysis["coverage"]["completed_fraction"] == 1.0
assert analysis["reuse_parity"]["pass"] is True
assert analysis["prefix_cost"]["pass"] is True
assert analysis["cost_ratio_prediction"]["pass"] is False
assert analysis["cost_ratio_prediction"]["campaigns_passing"] == 0
assert analysis["active_switching_authorized"] is False
assert analysis["gates"]["active_switching"] is False
print("G4_S4_VERIFICATION=PASS")
