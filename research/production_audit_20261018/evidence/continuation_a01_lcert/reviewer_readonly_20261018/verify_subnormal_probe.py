#!/usr/bin/env python3
from fractions import Fraction as F
from pathlib import Path
import json, runpy, struct, sys
oracle = runpy.run_path(sys.argv[1])
checks = 0
rows = 0
for line in Path(sys.argv[2]).read_text().splitlines():
    bits = list(map(int, line.split()))
    values = [struct.unpack("<d", struct.pack("<Q", b))[0] for b in bits[1:]]
    z = tuple(map(F.from_float, values[:2]))
    width, chi, time, eta = map(F.from_float, values[2:6])
    for perturb in ({F(0)} if eta == 0 else {F(0), eta, -eta}):
        initial = (z[0] + perturb, z[1])
        phase = (width / 2 + chi * (initial[0]**2 + initial[1]**2)) * time
        exact = oracle["rotate"](initial, phase)
        oracle["encloses_disk"](values[6:8], values[8], exact)
        oracle["encloses_disk"](values[9:11], values[11], exact)
        checks += 2
    rows += 1
assert rows == 6 and checks == 24
print(json.dumps({"status":"PASS", "rows":rows, "exact_fraction_disk_checks":checks}))
