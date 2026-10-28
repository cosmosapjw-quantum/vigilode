import json
from pathlib import Path


source = Path("research/production_audit_20261018/evidence/continuation_a07/validated.json")
validated = json.loads(source.read_text())
accuracy = validated["accuracy"]
contract = validated["contract"]

reference_gate = all(value <= 0.05 for value in accuracy["reference_spreads"].values())
assert reference_gate is accuracy["reference_gate"]
assert contract["reference_threshold"] == 0.05
assert contract["rtol"] == 1e-8
assert contract["atol"] == [1e-11] * 3
assert contract["output_points"] == 101
assert contract["span"] == [0, 40]
assert accuracy["arms"]["native"]["accuracy_gate"] is True
assert accuracy["arms"]["diffsol"]["accuracy_gate"] is False
assert accuracy["arm_intervals_overlap"] is False
assert accuracy["accuracy_overlap_gate"] is False
assert accuracy["arms"]["native"]["max_grid_wrms_distance_to_fine_radau"] == 0.33753007149457903
assert accuracy["arms"]["diffsol"]["max_grid_wrms_distance_to_fine_radau"] == 3.3026992500504733

print(
    json.dumps(
        {
            "campaign_rerun": False,
            "reference_spread_threshold": 0.05,
            "reference_gate": reference_gate,
            "arm_accuracy_threshold": 1.0,
            "native_arm_accuracy_gate": accuracy["arms"]["native"]["accuracy_gate"],
            "diffsol_arm_accuracy_gate": accuracy["arms"]["diffsol"]["accuracy_gate"],
            "arm_intervals_overlap": accuracy["arm_intervals_overlap"],
            "combined_accuracy_gate": accuracy["accuracy_overlap_gate"],
            "matched_speed": "HOLD",
        },
        sort_keys=True,
    )
)
