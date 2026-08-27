#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
CAMPAIGN = ROOT / "research/k0_stage_telemetry_20260827"
EVIDENCE = CAMPAIGN / "evidence"
AGGREGATE = CAMPAIGN / "aggregate"
PLOTS = CAMPAIGN / "plots"

SPEC = importlib.util.spec_from_file_location(
    "validate_k0_cells", CAMPAIGN / "schema/validate_k0_cells.py"
)
assert SPEC is not None and SPEC.loader is not None
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)


def load(path: pathlib.Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


class K0CampaignArtifactContracts(unittest.TestCase):
    def test_exact_campaign_aggregate_failure_ledger_and_plots(self) -> None:
        validation = VALIDATOR.validate_evidence_dir(EVIDENCE)
        self.assertEqual(validation["verdict"], "COMPLETE")
        self.assertEqual(validation["observed_cell_count"], 12)

        aggregate = load(AGGREGATE / "K0_STAGE_TELEMETRY_AGGREGATE.json")
        self.assertEqual(aggregate["schema"], "vigilode-k0-stage-telemetry-aggregate/v1")
        self.assertEqual(aggregate["claim_class"], "EXPLORATORY_NONAUTHORITATIVE")
        self.assertEqual(aggregate["verdict"], "COMPLETE")
        self.assertEqual(aggregate["cell_states"], validation["cell_states"])
        self.assertEqual(aggregate["expected_cell_count"], 12)
        self.assertEqual(aggregate["observed_cell_count"], 12)
        manifest = aggregate["cell_content_manifest"]
        self.assertEqual(len(manifest), 12)
        for name, digest in manifest.items():
            path = EVIDENCE / name
            self.assertTrue(path.is_file())
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), digest)
        self.assertEqual(set(aggregate["stage_totals_by_kernel"]), set(VALIDATOR.KERNELS))
        self.assertTrue(all(row["stage_receipts"] > 0 for row in aggregate["cells"]))
        self.assertNotIn("speedup", json.dumps(aggregate).lower())
        self.assertNotIn("wall_seconds", json.dumps(aggregate).lower())

        failures = load(AGGREGATE / "failure_ledger.json")
        self.assertEqual(failures["cell_states"], validation["cell_states"])
        self.assertEqual(failures["failures"], [])

        commands = load(AGGREGATE / "executed_commands.json")
        self.assertEqual(len(commands["commands"]), 12)
        identities = {
            (row["kernel_arm"], row["family"]) for row in commands["commands"]
        }
        self.assertEqual(
            identities,
            {(kernel, family) for kernel in VALIDATOR.KERNELS for family in VALIDATOR.FAMILIES},
        )
        self.assertTrue(all(row["exit_code"] == 0 for row in commands["commands"]))

        plot_manifest = load(PLOTS / "plot_manifest.json")
        self.assertEqual(
            {row["metric"] for row in plot_manifest["plots"]},
            {"work", "true_residual", "scaled_nonlinear_remainder", "rhs_novelty_angle", "output_acceptance"},
        )
        for row in plot_manifest["plots"]:
            path = PLOTS / row["file"]
            self.assertTrue(path.is_file())
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), row["sha256"])
            self.assertTrue(row["reading"])

        source_diff = (AGGREGATE / "source_diff.txt").read_text(encoding="utf-8")
        self.assertIn("source_base=e1124586a4029f86669e7489278c61ef676d61aa", source_diff)
        self.assertIn("forbidden_paths_changed=0", source_diff)


if __name__ == "__main__":
    unittest.main()
