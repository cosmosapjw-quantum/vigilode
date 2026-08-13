from __future__ import annotations
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "research/generic_regime_telemetry_v14s0/scripts/analyze_regime_oracle.py"
CAMPAIGNS = ROOT / "research/generic_parallel_exponential_v13/results/campaigns"
CANONICAL = ROOT / "research/generic_regime_telemetry_v14s0/results/G4_S0_ORACLE_MAP.json"

class G4S0AnalysisTests(unittest.TestCase):
    def test_canonical_contract(self):
        doc = json.loads(CANONICAL.read_text())
        self.assertEqual(doc["schema"], "generic-regime-telemetry-g4-s0-v1")
        self.assertFalse(doc["source"]["candidate_algorithms_modified"])
        self.assertFalse(doc["source"]["switching_activated"])
        self.assertEqual(doc["strict_summary"]["covered_cells"], 12)
        self.assertGreater(doc["strict_summary"]["switch_opportunity_cells"], 0)
        self.assertGreater(doc["strict_summary"]["aggregate_sum_cost_speedup_vs_rodas"], 1.0)
        self.assertGreater(doc["auto_summary"]["aggregate_sum_cost_speedup_vs_rodas"], doc["strict_summary"]["aggregate_sum_cost_speedup_vs_rodas"])
        self.assertFalse(doc["claim"]["single_free_feature_is_sufficient"])
        self.assertFalse(doc["claim"]["active_switching_authorized"])

    def test_analysis_is_byte_deterministic(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            subprocess.run([
                "python", str(SCRIPT), "--campaign-dir", str(CAMPAIGNS), "--output-dir", str(out)
            ], check=True, stdout=subprocess.DEVNULL)
            self.assertEqual(
                CANONICAL.read_bytes(),
                (out / "G4_S0_ORACLE_MAP.json").read_bytes(),
            )

if __name__ == "__main__":
    unittest.main()
