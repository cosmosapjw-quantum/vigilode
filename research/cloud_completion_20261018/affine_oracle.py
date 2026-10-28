"""Independent exact-rational oracle; only Python standard library required."""
import json
import math
from fractions import Fraction as F
from pathlib import Path

def upper(value):
    x = float(value)
    return math.nextafter(x, math.inf) if F(x) < value else x

cases = []
for n in [0, 1, 3, 17, 64, 129]:
    maps = [[0.875 + ((j * 13) % 9) / 32, ((j * 17) % 11) / 128] for j in range(n)]
    e = F(1, 8)
    bounds = [upper(e)]
    for gain, defect in maps:
        e = F(gain) * e + F(defect)
        bounds.append(upper(e))
    cases.append(dict(name=f"nonpower_and_power_{n}", maps=maps, initial=0.125, ceil_exact=bounds))
target = Path(__file__).parent / "evidence/affine_oracles.json"
target.write_text(json.dumps({"schema_version": 1, "method": "Fraction.from_float recurrence with directed final conversion", "cases": cases}, indent=2) + "\n")
print(json.dumps({"cases": len(cases), "prefixes": sum(len(c["ceil_exact"]) for c in cases), "output": str(target)}))
