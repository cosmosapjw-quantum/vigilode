import importlib.util
import json
from pathlib import Path

base = Path(__file__).resolve().parent
root = Path(__file__).resolve().parents[4]
validator_path = root / "tools/benchmarks/diffsol/a07/src/validate_report.py"
spec = importlib.util.spec_from_file_location("a07_validator", validator_path)
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)

cold = [json.loads(line) for line in (base / "cold.jsonl").read_text().splitlines()]
warm = [json.loads(line) for line in (base / "warm.jsonl").read_text().splitlines()]
alloc = [
    json.loads(line)
    for name in ("alloc_native.jsonl", "alloc_diffsol.jsonl")
    for line in (base / name).read_text().splitlines()
]
assert validator.validate_campaign_rows(cold + warm + alloc)["valid"]
cold_ratio = validator.timing_summary(cold, "single_cold_process")["diffsol_over_native_median_ratio"]
warm_ratio = validator.timing_summary(warm, "warm_abba")["diffsol_over_native_median_ratio"]
assert abs(cold_ratio - 0.9195676453507838) < 1e-15, cold_ratio
assert abs(warm_ratio - 1.4194023547437313) < 1e-15, warm_ratio

reference = json.loads((base / "reference.json").read_text())
grid = json.loads((base / "grid.json").read_text())["times"]
validator.validate_reference_center(reference, grid)
for label, center in (
    ("truncated", reference["reference_center"][:1]),
    ("different_full", [[0.0, 0.0, 0.0] for _ in grid]),
):
    hostile = dict(reference)
    hostile["reference_center"] = center
    try:
        validator.validate_reference_center(hostile, grid)
    except RuntimeError as exc:
        print(f"PASS rejected {label}: {exc}")
    else:
        raise AssertionError(f"accepted hostile {label} reference center")

validated = json.loads((base / "validated.json").read_text())
assert validated["campaign_validation"]["counts"] == {"allocator": 2, "cold": 20, "warm": 160}
assert validated["accuracy"]["reference_gate"] is True
assert validated["accuracy"]["arms"]["diffsol"]["accuracy_gate"] is False
assert validated["timing"][0]["diffsol_over_native_median_ratio"] == cold_ratio
assert validated["timing"][1]["diffsol_over_native_median_ratio"] == warm_ratio
print(f"PASS cold paired median: {cold_ratio:.16f}")
print(f"PASS warm block median: {warm_ratio:.16f}")
print("PASS strict campaign counts, reference gate, and expected Diffsol accuracy HOLD")
