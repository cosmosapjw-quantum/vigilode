#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
NODE = ROOT / "research" / "generic_regime_atlas_v20"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    analysis = json.loads((NODE / "results/G4_S5B0_ANALYSIS.json").read_text())
    state = json.loads((NODE / "state/DAG_STATUS.json").read_text())
    provenance = json.loads((NODE / "state/BASELINE_PROVENANCE.json").read_text())

    assert analysis["scientific_identity"] is True
    assert analysis["corpus"]["all_trajectories_successful"] is True
    assert analysis["gates"]["matrix_free_newton_free"] is True
    assert analysis["gates"]["both_methods_nontrivial_residence"] is True
    assert analysis["gates"]["e_k_residence_ge6_exists"] is True
    assert analysis["gates"]["entry_adjusted_oracle_gain_ge5pct"] is True
    assert analysis["gates"]["selector_research_continues"] is True
    assert analysis["gates"]["active_switching_authorized"] is False
    assert analysis["economics"]["e_first_transactional_speedup"] > 1.0
    assert analysis["winner_map"]["e_k_steps"] > 0
    assert analysis["winner_map"]["r_jf_steps"] > 0
    assert state["current_node"]["status"] == "COMPLETE_READ_ONLY"
    assert state["active_switching"] == "HOLD"

    for relative, expected in provenance["frozen_comparator_hashes"].items():
        assert sha256(ROOT / relative) == expected

    campaigns = sorted((NODE / "results/campaigns").glob("G4S5B0_CANONICAL_[1-7].json"))
    assert len(campaigns) == 7
    for path in campaigns:
        payload = json.loads(path.read_text())
        assert payload["switching_active"] is False
        assert payload["committed_method"] == "protected-sequential-matrix-free-rodas5p"
        assert all(row["success"] for row in payload["trajectories"])
        assert all(row["explicit_jacobian_builds"] == 0 for row in payload["trajectories"])
        assert all(row["direct_factorizations"] == 0 for row in payload["trajectories"])
        assert all(row["newton_iterations"] == 0 for row in payload["trajectories"])

    print("G4_S5B0_VERIFICATION=PASS")


if __name__ == "__main__":
    main()
